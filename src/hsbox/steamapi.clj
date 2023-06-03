(ns hsbox.steamapi
  (:require [hsbox.db :as db :refer [get-steam-api-key]]
            [hsbox.util :refer [current-timestamp]]
            [watt.user :refer [player-bans player-summaries]]
            [clojure.string :as str]
            [clojure.set :as set]
            [taoensso.timbre :as timbre]))

(timbre/refer-timbre)
; Time it's ok to have stale data for a steamid
(def steamids-stale-days 1)

(defn init-steam-stale-days [days]
  (def hsbox.steamapi/steamids-stale-days days))

(defn- get-steamids-info-from-api [steamids]
  (let [log-fail (fn [reply] (if (not= 200 (:status reply))
                               (warn "Steam API call failed" (str reply))))]
    (if (empty? steamids)
      {}
      (let [call-args (list :steamids (str/join "," steamids) :key (get-steam-api-key))
            bans (apply player-bans call-args)
            summaries (apply player-summaries call-args)
            add-deleted-steamids (fn [updates]
                                   (let [deleted (set/difference (set steamids) (set (keys updates)))]
                                     (into updates (map #(vector % {}) deleted))))]
        (log-fail bans)
        (log-fail summaries)
        ; Sleep 2s after two steam api calls
        (Thread/sleep 2000)
        (->>
          (concat (-> bans :body :players) (-> summaries :body :response :players))
          (reduce #(let [steamid-str (get %2 :steamid (get %2 :SteamId))]
                     (if (nil? steamid-str)
                       %
                       (let [steamid (Long/parseLong steamid-str)]
                         (assoc % steamid (select-keys
                                            (merge (get % steamid) %2)
                                            [:avatar :avatarfull :personaname :NumberOfVACBans :DaysSinceLastBan :NumberOfGameBans])))))
                  {})
          (add-deleted-steamids)
          (db/update-steamids))))))

(defn get-steamids-info-cached
  "Returns a map with the cached steamid info from the database (any steamids
  without cached data will be missing from the returned map)

  steamids must be a seq of Long"
  [steamids]
  (->>
    (db/get-steamid-info steamids)
    (filter #(> (:timestamp %) (- (current-timestamp) (* 24 3600 steamids-stale-days))))
    (reduce #(assoc % (:steamid %2) (dissoc %2 :steamid :timestamp)) {})))

(defn get-steamids-info [steamids & {:keys [refresh-all? delete-old?] :or {refresh-all? false delete-old? false}}]
  (assert (every? #(= Long %)
                  (map #(type %) steamids)))
  (if (not (str/blank? (get-steam-api-key)))
    (let [cached (if refresh-all?
                   []
                   (get-steamids-info-cached steamids))
          to-get (clojure.set/difference (set steamids) (set (keys cached)))
          _ (if (not (empty? to-get))
              (debug "Getting fresh data from the API for" (count to-get) "steamids"))
          from-api (apply merge (map get-steamids-info-from-api (partition 100 100 [] to-get)))]
      (if (or refresh-all? delete-old?)
        (db/delete-old-steamids))
      (merge cached from-api))
    {}))
