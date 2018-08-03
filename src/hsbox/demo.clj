(ns hsbox.demo
  (:require [clojure.set]
            [clojure.java.io :refer [input-stream as-file]]
            [clojure.java.shell :refer [sh]]
            [clojure.string :refer [split-lines split trim]]
            [clojure.data.json :as json]
            [hsbox.db :refer [kw-steamids-to-long latest-data-version]],
            [hsbox.util :refer [file-exists? last-modified]]
            [hsbox.stats :refer [add-round-numbers]]
            [flatland.protobuf.core :refer [protodef protobuf-load]]
            [taoensso.timbre :as timbre]))

(timbre/refer-timbre)
(import Cstrike15Gcmessages$CDataGCCStrike15_v2_MatchInfo)
(def MatchInfo (protodef Cstrike15Gcmessages$CDataGCCStrike15_v2_MatchInfo))
(def PARSER-CACHE nil)
(def demoinfo-dir-path (System/getProperty "user.dir"))

(defn read-file [file-path]
  (with-open [reader (input-stream file-path)]
    (let [length (.length (clojure.java.io/file file-path))
          buffer (byte-array length)]
      (.read reader buffer 0 length)
      buffer)))

(defn parse-mm-info-file [demo-path]
  (let [mm-info-path (str demo-path ".info")
        mm-info-file (as-file mm-info-path)]
    (if (file-exists? mm-info-file)
      (do
        (info "Processing" mm-info-path)
        (protobuf-load MatchInfo (read-file mm-info-path)))
      {})))

(defn parse-json-info-file [demo-path]
  (let [json-info-path (str demo-path ".json")
        json-info-file (as-file json-info-path)]
    (if (file-exists? json-info-file)
      (do
        (info "Processing" json-info-file)
        (->
          (slurp json-info-file)
          (json/read-str :key-fn keyword)))
      {})))

(defn get-parser-cache []
  (get (System/getenv) "HEADSHOTBOX_PARSER_CACHE" PARSER-CACHE))

(defn set-demoinfo-dir [dir]
  (def demoinfo-dir-path dir))

(defn parse-demo [path]
  (let [json-cache (get-parser-cache)
        json-path (str json-cache "/" (.getName (as-file path)) ".json")
        do-parse (fn []
                   (let [proc (clojure.java.shell/sh (str demoinfo-dir-path "/demoinfogo") path "-hsbox")]
                     (assert (zero? (:exit proc)) (:err proc))
                     (:out proc)))]
    (if (nil? json-cache)
      (do-parse)
      ; Use the cache, Luke!
      (do
        (when (not (.exists (as-file json-path)))
          (spit json-path (do-parse)))
        (slurp json-path)))))

(defn get-demo-type [demo]
  (letfn [(has_gotv_bot [name] (some #(.contains % name) (:gotv_bots demo)))]
    (cond
      (.contains (:servername demo) "Valve") "valve"
      (.contains (:servername demo) "CEVO") "cevo"
      (.contains (:servername demo) "GamersClub") "gamersclub"
      (.contains (:servername demo) "PCMR") "faceit"
      (has_gotv_bot "ESEA") "esea"
      (or (has_gotv_bot "FACEIT GOTV") (.contains (:servername demo) "FACEIT.com")) "faceit")))

(defn split-by-game-restart [demo-events]
  (->> demo-events
       (partition-by #(= (:type %) "game_restart"))
       (filter #(not= "game_restart" (:type (first %))))))

; A long time ago, the event round_officially_ended didn't exist
; So use this sad weird heuristic which seems to work
(defn split-by-round-end [chunk]
  (loop [rounds [] events chunk]
    (if (empty? events)
      rounds
      (let [[a b] (split-with #(not= (:type %) "round_end") events)
            [c d] (split-with #(#{"round_end" "player_death" "score_changed"} (:type %)) b)
            [e _] (split-with #(not= (:type %) "round_start") d)
            late-score-changed-events (filter #(= (:type %) "score_changed") e)
            ; ignore score_changed before round_end
            before-end (vec (filter #(not= (:type %) "score_changed") a))
            round-events (concat before-end c late-score-changed-events)]
        (recur (conj rounds round-events) d)))))

; round_end events sometimes are missing and hopefully round_officially_ended doesn't
(defn split-by-round-officially-ended [chunk]
  (loop [rounds [] events chunk]
    (if (empty? events)
      rounds
      (let [[a b] (split-with #(not= (:type %) "round_officially_ended") events)]
        (recur (conj rounds a) (rest b))))))

(defn process-round-events [events]
  (letfn [(process
            [round event]
            (case (:type event)
              "round_start" (assoc round :tick (:tick event))
              "score_changed" (assoc round :score_changed (:score event)
                                           :score_changed_tick (:tick event))
              "round_end" (assoc round :tick_end (:tick event)
                                       :winner (:winner event)
                                       :win_reason (:reason event))
              "player_hurt" (let [health (get-in round [:health (:userid event)] 100)
                                  damage (min (:dmg_health event) health)
                                  team (fn [steamid]
                                         (get-in round [:players steamid]))
                                  update-dmg (fn [round]
                                               (if (and (not= 0 (:attacker event)) (not= (team (:userid event)) (team (:attacker event))))
                                                 (update-in round [:damage (:attacker event)]
                                                            #(if (nil? %) %2 (+ % %2)) damage)
                                                 round))]
                              (-> round
                                  (assoc-in [:health (:userid event)] (:health event))
                                  (update-dmg)))
              "player_spawn" (if (or (= 0 (:teamnum event)) (= 0 (:userid event)))
                               round
                               (assoc-in round [:players (:userid event)] (:teamnum event)))
              "player_death" (let [death (conj
                                           (select-keys event '(:assister :attacker :headshot :penetrated :tick :weapon
                                                                 :jump :smoke :attacker_pos :victim_pos :scoped_since :air_velocity))
                                           {:victim (:userid event)})]
                               (update-in round [:deaths] conj death))
              "bomb_defused" (assoc round :bomb_defused (:userid event))
              "bomb_exploded" (assoc round :bomb_exploded (:userid event))
              ; disconnected players are interesting only when they have spawned and haven't died
              "player_disconnected" (if (and
                                          (get-in round [:players (:userid event)])
                                          (empty? (filter #(= (:victim %) (:userid event)) (:deaths round))))
                                      (assoc-in round [:disconnected (:userid event)] (:tick event))
                                      round)
              round))]
    ; Filter events before round start tick
    (let [round-tick (:tick (last (filter #(= (:type %) "round_start") events)))]
      (reduce process {:players {} :deaths [] :damage {} :health {} :disconnected {}} (filter #(<= round-tick (:tick %)) events)))))

(defn has-event [events type]
  (some #(= (:type %) type) events))

(defn filter-esea-possible-rounds [possible-rounds]
  ; Score doesn't get updated after the last round in ESEA
  (conj (vec (filter #(has-event % "score_changed") (butlast possible-rounds))) (last possible-rounds)))

(def filter-possible-rounds {"esea" filter-esea-possible-rounds})

; This gets rid of knife rounds as ESEA has 2 minutes for it (wtf?) and faceit 1 hour
(defn round-timelimit [demo-type]
  (if (= "valve" demo-type)
    120
    115))

; Legit rounds have:
; - round_start with legit timelimit
; - either non-draw round_end or the last score_changed happens at least 2s after round_start
;   (should be 15s (buytime) but I'm too lazy to pass tickrate here;
;    anyway, uninteresting score_changed happen quickly after the round_start: 0-0 or team switch)
(defn get-rounds [demo-events demo-type]
  (let
    [round-split-func (if (some #(= (:type %) "round_officially_ended") demo-events)
                        split-by-round-officially-ended
                        split-by-round-end)
     split-at-last-round-start (fn [events]
                                 (split-with #(not= (:type %) "round_start") (reverse events)))]
    (vec
      (->> demo-events
           (split-by-game-restart)
           ; mapcat?
           (map round-split-func)
           (apply concat)
           ; Remove chunks with no legit start_round event
           (filter (fn [events]
                     (not (empty? (filter #(and (= (:type %) "round_start") (<= 105 (:timelimit %) (round-timelimit demo-type))) events)))))
           ; Remove score_changed events that happen in max 2 seconds after round_start
           (map (fn [events]
                  (let [[after-start before-start] (split-at-last-round-start events)
                        start-tick (:tick (first before-start))]
                    (reverse (concat
                               (filter #(or (not= (:type %) "score_changed")
                                            (and (= (:type %) "score_changed") (< (+ 256 start-tick) (:tick %))))
                                       after-start)
                               before-start)))))
           ; Filter chunks with no round_end or score_changed
           (filter (fn [events]
                     (let [[after-start _] (split-at-last-round-start events)]
                       (true?
                         (or
                           ; Draw rounds can only happen in warmup
                           (not (empty? (filter #(and (= (:type %) "round_end") (not= 1 (:winner %))) after-start)))
                           (not (empty? (filter #(= (:type %) "score_changed") after-start))))))))
           ((get filter-possible-rounds demo-type identity))
           (map process-round-events)
           (map #(dissoc % :health))))))

(defn real-team [demo team]
  (if (:teams_switched? demo)
    (- 5 team)
    team))

(defn isHuman? [steamid]
  (> steamid 76561197960265728))

(defn update-players [demo player team]
  (let [initial-team (get-in demo [:players player :team])]
    (if (isHuman? player)
      (if initial-team
        ; Check if teams switched
        (assoc demo :teams_switched? (not= team initial-team))
        ; Add new player
        (assoc-in demo [:players player] {:name (get-in demo [:player_names player])
                                          :team (real-team demo team)}))
      demo)))

(defn total-score [demo]
  (vec (apply map + (:detailed_score demo))))

(defn update-score [demo round]
  (let [score (:detailed_score demo)
        round-end-missing (nil? (:winner round))
        score-changed (if (:teams_switched? demo)
                        (reverse (:score_changed round))
                        (:score_changed round))
        ; Hack needed when round_end event is missing
        ; Compare the score_changed event against the one from last round
        guess-winner (fn []
                       (real-team demo
                         (if (nil? (:last_score_changed demo))
                           (do
                             (assert (and (= 1 (apply + score-changed)) (#{0 1} (first score-changed)) (#{0 1} (second score-changed))))
                             (if (= 1 (first score-changed))
                               2
                               3))
                           (if (< (first (:last_score_changed demo)) (first score-changed))
                             2
                             3))))
        round-winner (if (not round-end-missing)
                       (:winner round)
                       (guess-winner))]
    (-> demo
        (assoc :detailed_score
               (conj
                 (vec (butlast score))
                 (update-in (last score) [(- (real-team demo round-winner) 2)] inc))
               :last_score_changed
               score-changed)
        ; Check if computed score differs from score_changed
        ((fn [demo]
           (do
             (if (and (not (nil? (:score_changed round)))
                      (not= (apply + score-changed) (apply + (total-score demo)))
                      (not= (count (:rounds demo)) (:number round)))
               (warn "Total number of rounds differs in demo " (:path demo) ": computed score" (:detailed_score demo)
                     " from score_changed events " score-changed))
             demo)))
        (update-in [:rounds (dec (:number round))]
                   #(-> %
                        (conj (when round-end-missing [:winner round-winner]))
                        (conj (when round-end-missing [:tick_end (:score_changed_tick round)]))
                        (dissoc :score_changed :score_changed_tick))))))

(defn check-ot-half-started [mr round_no]
  (if (or (nil? mr) (<= round_no 30))
    false
    (= 1 (mod (- round_no 30) mr))))

(defn update-winner [demo]
  (let [score (total-score demo)
        [a_wins b_wins] score]
    (-> demo
        (assoc :score score
               :winner (if (not (:surrendered demo))
                         (cond
                           (< a_wins b_wins) 3
                           (> a_wins b_wins) 2
                           :else 0)
                         (real-team demo (:winner demo)))))))

(defn enrich-with-round [demo round]
  (->
    ; Update players table
    (reduce-kv update-players demo (:players round))
    ; Detect overtime MR6/MR10
    (#(if (and (not= (get % :teams_switched?) (:teams_switched? demo)) (#{34 36} (:number round)))
       (assoc % :mr (- (:number round) 31))
       %))
    ; Start a new score table if overtime started (round 31 - we still don't know MR rules at this point)
    ; or teams switched or we're in overtime but teams switched last half
    (#(if (or (= (:number round) 31)
              (not= (get % :teams_switched?) (:teams_switched? demo))
              (check-ot-half-started (:mr demo) (:number round)))
       (update-in % [:detailed_score] conj [0 0])
       %))
    (update-score round)))

(defn enrich-demo [demo]
  (let [demo (merge demo {:players         {}
                          :detailed_score  [[0 0]]
                          :teams_switched? false})
        enriched (doall
                   (-> (reduce enrich-with-round demo (add-round-numbers (:rounds demo)))
                       (update-winner)
                       (dissoc :teams_switched? :player_names :last_score_changed :path)))]
    (assert (not (or (empty? (:rounds enriched)) (empty? (:players enriched))))
            (str "Demo " (:path demo) " has " (count (:rounds enriched)) " rounds and " (count (:players enriched)) " players"))
    enriched))

(defn get-demo-info [path]
  (info "Processing" path)
  (let [demo-data (->>
                    (json/read-str (parse-demo path) :key-fn keyword)
                    (kw-steamids-to-long [:player_names])
                    (kw-steamids-to-long [:player_slots])
                    (kw-steamids-to-long [:mm_rank_update]))
        demo-type (get-demo-type demo-data)
        ; Parse MM dem.info file if available (for demo timestamp)
        scoreboard (try
                     (parse-mm-info-file path)
                     (catch Throwable e {}))
        last-round-end (last (filter #(= (:type %) "round_end") (:events demo-data)))
        surrendered? (if last-round-end
                       (.contains (:message last-round-end) "Surrender")
                       false)
        ; If winner gets set here, it will be rewritten when we know if on the last rounds teams were switched
        winner (if surrendered?
                 (:winner last-round-end)
                 0)
        rounds (get-rounds (:events demo-data) demo-type)
        demo (merge {:rounds      rounds
                     :path        path
                     :type        demo-type
                     :timestamp   (get scoreboard :matchtime (last-modified path))
                     :surrendered surrendered?
                     :winner      winner}
                    (select-keys demo-data [:map :player_names :tickrate :mm_rank_update :player_slots]))
        ; Parse dem.json info file if available (for demo timestamp and metrics)
        json-info (try
                    (parse-json-info-file path)
                    (catch Throwable e {}))
        demo (merge demo json-info)]
    (if (not (contains? latest-data-version demo-type))
      (throw (Exception. "Unknown demo type")))
    (enrich-demo demo)))
