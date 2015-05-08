(ns hsbox.demo
  (:require [clojure.set]
            [clojure.java.io :refer [input-stream as-file]]
            [clojure.java.shell :refer [sh]]
            [clojure.string :refer [split-lines split trim]]
            [clojure.data.json :as json]
            [hsbox.db :refer [kw-steamids-to-long]],
            [hsbox.util :refer [file-exists?]]
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
        (info "Processing" mm-info-file)
        (protobuf-load MatchInfo (read-file mm-info-path)))
      {})))

(defn get-demo-id [path]
  (.getName (clojure.java.io/file path)))

(defn parse-demo [path]
  (let [proc (clojure.java.shell/sh (str (System/getProperty "user.dir") "/demoinfogo") path "-hsbox")]
    (assert (zero? (:exit proc)))
    (->>
      (json/read-str (:out proc) :key-fn keyword)
      (kw-steamids-to-long [:players]))))

(defn process-event [rounds event]
  "The :fake property is true if there are two round_end events in row without a round_start in between.
  This is needed because surrendering always generates a round_end event."
  (let [add-stuff-to-current-round (fn [f]
                                     (let [round (first rounds)]
                                       (conj (pop rounds) (f round))))]
    (case (:type event)
      "round_start" (conj rounds {:tick (:tick event)})
      "round_end" (let [round-end-info {:winner (:winner event) :win_reason (:reason event) :tick_end (:tick event)}]
                    (if (get (first rounds) :winner)
                      (conj rounds (assoc round-end-info :fake true))
                      (add-stuff-to-current-round
                        #(merge % round-end-info))))
      "player_death" (add-stuff-to-current-round
                       (let [death (conj
                                     (select-keys event '(:assister :attacker :headshot :penetrated :tick :weapon))
                                     {:victim (:userid event)})]
                         #(assoc % :deaths (conj (vec (:deaths %)) death))))
      rounds)))

(defn process-events [events]
  (reverse (reduce process-event '() (drop-while #(not (and (= (:type %) "round_start") (= (:timelimit %) 120))) events))))

(defn compute-score [rounds]
  (let [real-rounds (fn [rounds] (filter #(not (get % :fake false)) rounds))
        half-score (fn [rounds]
                     (let [rounds-won (fn [team] (count (filter #(= (:winner %) team) (real-rounds rounds))))]
                       [(rounds-won 2) (rounds-won 3)]))
        true-team (fn [team round-number] (if (<= round-number 15) team (- 5 team)))
        first-half (half-score (take 15 rounds))
        second-half (vec (reverse (half-score (drop 15 rounds))))
        score (map + first-half second-half)
        surrendered (contains? #{16 17} (get (last rounds) :win_reason))
        winner (if surrendered
                 (true-team (:winner (last rounds)) (count (real-rounds rounds)))
                 (cond
                   (> (first score) (second score)) 2
                   (< (first score) (second score)) 3
                   :default 0))]
    {:first_half first-half :second_half second-half :score score :surrendered surrendered :winner winner}))

(defn get-demo-info [path]
  (info "Processing" path)
  (let [demo-data (parse-demo path)
        ; Parse MM dem.info file if available (for demo timestamp)
        scoreboard (try
                     (parse-mm-info-file path)
                     (catch Exception e {}))
        rounds (process-events (:events demo-data))
        score (compute-score rounds)]
    (merge {:rounds (filter #(not (:fake %)) rounds)
            :score score}
           (select-keys demo-data [:map :players :tickrate])
           {:timestamp (get scoreboard :matchtime (/ (.lastModified (as-file path)) 1000))})))
