(ns hsbox.launch
  (:require [hsbox.stats :as stats])
  (:require [hsbox.util :refer [file-exists? file-name]])
  (:require [hsbox.db :as db])
  (:require [hsbox.version :refer [os-name]])
  (:require [hsbox.env :as env])
  (:require [clojure.java.io :as io])
  (:require [clojure.string :as str]))

(taoensso.timbre/refer-timbre)

(def HEADSHOTBOX-WATERMARK "// Generated by Headshot Box")

(defn generated-by-hsbox [vdm-path]
  (.startsWith (slurp vdm-path) HEADSHOTBOX-WATERMARK))

(defn- append-maybe [x pred xs]
  (if pred
    (conj x xs)
    x))

(defn- sec-to-tick [demo sec]
  (stats/seconds-to-ticks sec (:tickrate demo)))

(defn- fade-to-black [tick]
  {:factory   "ScreenFadeStart"
   :tick      tick
   :duration  "1.000"
   :holdtime  "1.000"
   :FFADE_IN  "1"
   :FFADE_OUT "1"
   :r         "0"
   :g         "0"
   :b         "0"
   :a         "255"})


(defn- generate-highlight-enemy-pov [demo kill]
  (let [kill-context (sec-to-tick demo 5)
        after-kill-context (sec-to-tick demo 2)]
    (-> []
        (append-maybe
          (or (= (:tick-before kill) 0)
              (> (- (:tick kill) (:tick-before kill)) (+ kill-context after-kill-context)))
          {:factory    "SkipAhead"
           :tick       (if (= 0 (:tick-before kill))
                         0
                         (+ (:tick-before kill) after-kill-context))
           :skiptotick (- (:tick kill) kill-context)})

        (append-maybe true
                      {:factory  "PlayCommands"
                       :tick     (max (- (:tick kill) kill-context) (:tick-before kill))
                       :commands (str "spec_player_by_accountid " (:victim kill))})
        (append-maybe
          (> (- (:tick-after kill) (:tick kill)) (sec-to-tick demo 1))
          (fade-to-black (:tick kill))))))

(defn- quit-or-disconnect []
  (if (:vdm_quit_after_playback (db/get-config))
    "quit"
    "disconnect"))

(defn- vdm-highlights [demo steamid]
  (let [killed-by-steamid (fn [kill] (= steamid (:attacker kill)))
        kills (mapcat #(filter killed-by-steamid (:deaths %)) (:rounds demo))
        tick-before (conj (map #(:tick %) kills) 0)
        tick-after (conj (vec (map #(:tick %) (rest kills))) (+ (:tick (last kills)) 9999))
        augmented-kills (map #(assoc %3 :tick-before % :tick-after %2) tick-before tick-after kills)
        cfg (:vdm_cfg (db/get-config))]
    {:tick 0
     :vdm  (-> []
               (append-maybe (not (empty? cfg))
                             {:factory  "PlayCommands"
                              :tick     0
                              :commands (str "exec " cfg)})
               (into (mapcat #(generate-highlight-enemy-pov demo %) augmented-kills))
               (append-maybe true {:factory  "PlayCommands"
                                   :tick     (+ (:tick (last augmented-kills)) (stats/seconds-to-ticks 1 (:tickrate demo)))
                                   :commands (quit-or-disconnect)}))}))

(defn- generate-pov [demo round steamid]
  (let [death (first (filter #(= (:victim %) steamid) (:deaths round)))
        tick-jump (if death
                    (+ (:tick death) (sec-to-tick demo 3))
                    (+ (:tick_end round) (sec-to-tick demo 5)))]
    (if (nil? (:next-round-tick round))
      [{:factory  "PlayCommands"
        :tick     tick-jump
        :commands (quit-or-disconnect)}]
      [(fade-to-black (- tick-jump (sec-to-tick demo 1)))
       {:factory    "SkipAhead"
        :tick       tick-jump
        :skiptotick (+ (:next-round-tick round) (sec-to-tick demo 15))}])))


(defn vdm-pov [demo steamid]
  (let [cfg (:vdm_cfg (db/get-config))
        rounds (filter #(get (:players %) steamid) (:rounds demo))
        tick-after (conj (vec (map #(:tick %) (rest rounds))) nil)
        augmented-rounds (map #(assoc %2 :next-round-tick %) tick-after rounds)]
    {:tick 0
     :vdm  (-> []
               (append-maybe (not (empty? cfg))
                             {:factory  "PlayCommands"
                              :tick     0
                              :commands (str "exec " cfg)})
               (append-maybe true {:factory  "PlayCommands"
                                   :tick     0
                                   :commands (str "spec_player_by_accountid " steamid)})
               (append-maybe true
                             {:factory    "SkipAhead"
                              :tick       0
                              :skiptotick (+ (:tick (first augmented-rounds)) (sec-to-tick demo 15))})

               (into (mapcat #(generate-pov demo % steamid) augmented-rounds)))}))

(defn escape-path [path] (str "\"" path "\""))

(defn vdm-round-highlights [demo steamid round-number kill-filter]
  (let [round (nth (:rounds demo) (dec round-number))
        _ (println "in vdm-round-highlights " (get demo :demoid))
        kills (filter #(= (:attacker %) steamid) (filter kill-filter (:deaths round)))
        _ (assert (not (empty? kills)))
        context-before-first-kill (sec-to-tick demo 5)
        context-after-last-kill (sec-to-tick demo 3)
        context-before-kill (sec-to-tick demo 3)
        context-after-kill (sec-to-tick demo 1)
        close-kills (sec-to-tick demo 3)
        ; TODO (:tick round) for movie
        start-tick (- (:tick (first kills)) context-before-first-kill)
        kill-pairs (map vector kills (rest kills))
        clip-prefix (fn [number] (str (get demo :demoid) "_" steamid "_" round-number "_" number))
        start-command (fn [& args] {:factory  "PlayCommands"
                                    :tick     start-tick
                                    :commands (apply str args)})
        start-movie-command (fn [tick] {:factory  "PlayCommands"
                                        :tick     tick
                                        :commands "mirv_streams record start"})
        stop-movie-command (fn [tick] {:factory  "PlayCommands"
                                       :tick     tick
                                       :commands "mirv_streams record end"})
        clips-info (reduce
                     #(let [a (:tick (first %2))
                            b (:tick (second %2))]
                        (if (> (- b a) (+ context-after-kill context-before-kill close-kills))
                          (-> %
                              (assoc :commands (conj (:commands %)
                                                     (stop-movie-command (+ a context-after-kill))
                                                     (start-movie-command (- b context-before-kill))))
                              (assoc :clip-ids (conj (:clip-ids %) (clip-prefix (count (:clip-ids %))))))
                          %))
                     {:commands [(start-movie-command (- (:tick (first kills)) context-before-first-kill))
                                 (stop-movie-command (+ (:tick (last kills)) context-after-last-kill))]
                      :clip-ids [(clip-prefix 0)]}
                     kill-pairs)
        user-id (get-in round [:userid steamid])
        _ (println clips-info)
        _ (debug "Round" round steamid user-id)
        entity-id (inc (get-in demo [:player_slots steamid]))]
    {:tick     start-tick
     :vdm      (-> [(start-command "exec movie")
                    (start-command "spec_player " entity-id)
                    (start-command "mirv_deathmsg block !" user-id " *")
                    (start-command "mirv_deathmsg highLightId " user-id)
                    (start-command "spec_show_xray 0")
                    (start-command (str "mirv_streams record name " (escape-path env/raw-data-folder)))
                    {:factory  "PlayCommands"
                     :tick     (+ (:tick (last kills)) context-after-last-kill 50)
                     :commands "quit"}]
                   ; TODO more context for nades
                   ; TODO slowmo for jumpshot
                   (into (filter identity
                                 (apply concat
                                        (map
                                          ; TODO don't set it on if collateral and penetrated?
                                          #(if (or (>= (:penetrated %) 1) (:smoke %))
                                             [{:factory  "PlayCommands"
                                               :tick     (- (:tick %) (sec-to-tick demo 1))
                                               :commands "spec_show_xray 1"}
                                              {:factory  "PlayCommands"
                                               :tick     (+ (:tick %) (sec-to-tick demo 0.3))
                                               :commands "spec_show_xray 0"}])
                                          kills))))
                   (into (:commands clips-info)))
     :clip-ids (:clip-ids clips-info)}))

(defn vdm-watch [demo steamid tick tick-end]
  (let [user-id (get (:player_slots demo) steamid 0)
        cfg (:vdm_cfg (db/get-config))]
    {:tick tick
     :vdm  (-> []
               ; spec_player seems to be working more often than spec_player_by_accountid
               (append-maybe (:player_slots demo)
                             {:factory  "PlayCommands"
                              :tick     (or tick 0)
                              :commands (str "spec_player " (inc user-id))})
               ; but spec_player_by_accountid works without player_slots, so we'll keep both
               (append-maybe true {:factory  "PlayCommands"
                                   :tick     (or tick 0)
                                   :commands (str "spec_player_by_accountid " steamid)})
               ; spec_lock also, cause why not? (doesn't seem to work though)
               ;(append-maybe true {:factory  "PlayCommands"
               ;                    :tick     (or tick 0)
               ;                    :commands (str "spec_lock_to_accountid " steamid)})
               (append-maybe (not (empty? cfg))
                             {:factory  "PlayCommands"
                              :tick     (or tick 0)
                              :commands (str "exec " cfg)})
               (append-maybe tick-end
                             {:factory  "PlayCommands"
                              :tick     tick-end
                              :commands (quit-or-disconnect)}))}))

(defn generate-command [number command]
  (let [line (fn [key value] (str "\t\t" key " \"" (str/escape (str value) {\" "\\\"" \\ "\\\\"}) "\"\n"))
        content (apply str (map #(line (name (first %)) (second %))
                                (-> command
                                    (assoc :starttick (:tick command)
                                           :name ".")
                                    (dissoc :tick))))]
    (str "\t\"" number "\"\n"
         "\t{\n"
         content
         "\t}\n")))

(defn generate-vdm [commands]
  (str HEADSHOTBOX-WATERMARK
       "\ndemoactions\n{\n"
       (apply str
              (mapv #(generate-command (first %) (second %))
                    (map vector (rest (range)) (sort #(compare (:tick %) (:tick %2)) commands))))
       "}\n"))

(defn delete-vdm [vdm-path]
  (debug "Deleting vdm file" vdm-path)
  (io/delete-file vdm-path true))

(defn kill-csgo-process []
  (if (= os-name "windows")
    (clojure.java.shell/sh "taskkill" "/im" "csgo.exe" "/F")
    (clojure.java.shell/sh "killall" "-9" "csgo_linux")))

(defn write-vdm-file
  "Write VDM file if needed and return start tick"
  [demo steamid tick round-number highlight & {:keys [kill-filter] :or {kill-filter (fn [_] true)}}]
  (let [demo-path (:path demo)
        _ (println "in write-vdm-file" (get demo :demoid))
        vdm-path (str (subs demo-path 0 (- (count demo-path) 4)) ".vdm")
        config (db/get-config)]
    (when
      (and (not (:vdm_enabled config))
           (file-exists? vdm-path))
      (delete-vdm vdm-path))
    (when (and
            (:vdm_enabled config)
            (file-exists? demo-path))
      (if (and (#{"high" "low"} highlight))
        (when (file-exists? vdm-path)
          (delete-vdm vdm-path))
        (do
          (debug "Writing vdm file" vdm-path)
          (let [vdm-info (case highlight
                           "high_enemy" (vdm-highlights demo steamid)
                           "pov" (vdm-pov demo steamid)
                           "round" (vdm-round-highlights demo steamid round-number kill-filter)
                           (vdm-watch demo steamid tick
                                      (when round-number (+ (:tick_end (nth (:rounds demo) (dec round-number)))
                                                            (stats/seconds-to-ticks 5 (:tickrate demo))))))]
            (spit vdm-path (generate-vdm (:vdm vdm-info)))
            vdm-info))))))

(defn watch [local? demoid steamid round-number tick highlight]
  (let [demo (get stats/demos demoid)
        demo-path (:path demo)
        vdm-path (str (subs demo-path 0 (- (count demo-path) 4)) ".vdm")
        play-path (if local? demo-path (str "replays/" (file-name demo-path)))]
    (if (nil? demo)
      ""
      (do
        (when round-number
          (assert (<= 1 round-number (count (:rounds demo)))))
        (let [round (when round-number (nth (:rounds demo) (dec round-number)))
              tick (if round
                     (+ (:tick round)
                        (stats/seconds-to-ticks 15 (:tickrate demo)))
                     tick)
              tick (if local?
                     (:tick (write-vdm-file demo steamid tick round-number highlight))
                     tick)]
          ; VDM works only with local requests
          (when local?
            (when
              (and (not (:vdm_enabled (db/get-config)))
                   (file-exists? vdm-path))
              (delete-vdm vdm-path))
            (when (and
                    (:vdm_enabled (db/get-config))
                    (file-exists? demo-path))
              (if (and (#{"high" "low"} highlight))
                (when (file-exists? vdm-path)
                  (delete-vdm vdm-path))
                (do
                  (debug "Writing vdm file" vdm-path))))
            ;(spit vdm-path (generate-vdm
            ;                 (case highlight
            ;                   "high_enemy" (vdm-highlights demo steamid)
            ;                   "pov" (vdm-pov demo steamid)
            ;                   (vdm-watch demo steamid tick
            ;                              (when round (+ (:tick_end round)
            ;                                             (stats/seconds-to-ticks 5 (:tickrate demo))))))))

            (when (and (:playdemo_kill_csgo (db/get-config)))
              (kill-csgo-process))
            (clojure.java.shell/sh env/steam-path "-applaunch" "730" "+playdemo" (str play-path (when tick (str "@" tick)) " "
                                                                                      (when (#{"high" "low"} highlight) steamid)
                                                                                      (when (= highlight "low") " lowlights")))))))))

(defn delete-generated-files []
  (let [path (db/get-demo-directory)]
    (->> (clojure.java.io/as-file path)
         file-seq
         (map #(when (and (.endsWith (.getName %) ".vdm") (generated-by-hsbox %))
                 (delete-vdm (.getAbsolutePath %))))
         dorun)))
