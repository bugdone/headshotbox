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

(defn get-demo-id [path]
  (.getName (clojure.java.io/file path)))

(defn parse-demo [path]
  (let [proc (clojure.java.shell/sh (str (System/getProperty "user.dir") "/demoinfogo") path "-hsbox")]
    (assert (zero? (:exit proc)))
    (->>
      (json/read-str (:out proc) :key-fn keyword)
      (kw-steamids-to-long [:player_names]))))

(defn get-demo-type [demo]
  (letfn [(has_gotv_bot [name] (some #(.contains % name) (:gotv_bots demo)))]
    (cond
      (.contains (:servername demo) "Valve") "valve"
      (.contains (:servername demo) "CEVO") "cevo"
      (has_gotv_bot "ESEA") "esea"
      (has_gotv_bot "FACEIT GOTV") "faceit")))

(defn split-by-game-restart [demo]
  (->> (:events demo)
       (partition-by #(= (:type %) "game_restart"))
       (filter #(not= "game_restart" (:type (first %))))))

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

(defn process-round-events [events]
  (letfn [(process
            [round event]
            (case (:type event)
              "round_start" (assoc round :tick (:tick event))
              "round_end" (assoc round :tick_end (:tick event)
                                       :winner (:winner event)
                                       :win_reason (:reason event))
              "player_spawn" (if (or (= 0 (:teamnum event)) (= 0 (:userid event)))
                               round
                               (assoc-in round [:players (:userid event)] (:teamnum event)))
              "player_death" (let [death (conj
                                           (select-keys event '(:assister :attacker :headshot :penetrated :tick :weapon :jump))
                                           {:victim (:userid event)})]
                               (update-in round [:deaths] conj death))
              round))]
    ; Filter events before round start tick
    (let [round-tick (:tick (last (filter #(= (:type %) "round_start") events)))]
      (reduce process {:players {} :deaths []} (filter #(<= round-tick (:tick %)) events)))))

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
    105))

(defn get-rounds [demo-data demo-type]
  (->> demo-data
       (split-by-game-restart)
       (map split-by-round-end)
       (apply concat)
       (filter (fn [events]
                 (and
                   (some #(and (= (:type %) "round_start") (<= 105 (:timelimit %) (round-timelimit demo-type))) events)
                   ; Draw rounds can only happen in warmup
                   (some #(and (= (:type %) "round_end") (not= 1 (:winner %))) events))))
       ((get filter-possible-rounds demo-type identity))
       (map process-round-events)))

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

(defn update-score [demo round]
  (let [score (:detailed_score demo)]
    (assoc demo :detailed_score
                (conj
                  (vec (butlast score))
                  (update-in (last score) [(- (real-team demo (:winner round)) 2)] inc)))))

(defn check-ot-half-started [mr round_no]
  (if (or (nil? mr) (<= round_no 30))
    false
    (= 1 (mod (- round_no 30) mr))))

(defn update-winner [demo]
  (let [score (vec (apply map + (:detailed_score demo)))
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
    ; Detect MR
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
    (update-score round)
    (dissoc :round_no)))

(defn enrich-demo [demo]
  (let [demo (merge demo {:players         {}
                          :detailed_score  [[0 0]]
                          :teams_switched? false})]
    (-> (reduce enrich-with-round demo (add-round-numbers (:rounds demo)))
        (update-winner)
        (dissoc :teams_switched? :player_names))))

(defn get-demo-info [path]
  (info "Processing" path)
  (let [demo-data (parse-demo path)
        demo-data (kw-steamids-to-long [:mm_rank_update] demo-data)
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
        rounds (get-rounds demo-data demo-type)
        demo (merge {:rounds      rounds
                     :type        demo-type
                     :timestamp   (get scoreboard :matchtime (last-modified path))
                     :surrendered surrendered?
                     :winner      winner}
                    (select-keys demo-data [:map :player_names :tickrate :mm_rank_update]))]
    (if (not (contains? latest-data-version demo-type))
      (throw (Exception. "Unknown demo type")))
    (enrich-demo demo)))