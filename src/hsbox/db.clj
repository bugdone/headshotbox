(ns hsbox.db
  (:require [clojure.data.json :as json]
            [clojure.java.jdbc :as jdbc]
            [clojure.java.io :as io :refer [resource]]
            [clojure.string :as str]
            [hsbox.util :refer [current-timestamp file-exists?]]
            [taoensso.timbre :as timbre])
  (:import (java.io File)))

(timbre/refer-timbre)
(def latest-data-version 1)
;(set! *warn-on-reflection* true)

(def app-config-dir
  (let [config-home (if-let [xdg (System/getenv "XDG_CONFIG_HOME")]
                      (File. xdg)
                      (File. (System/getProperty "user.home") ".config"))
        app-config (File. config-home "headshotbox")]
    (.mkdir app-config)
    app-config))

(def db
  {:classname "org.sqlite.JDBC"
   :subprotocol "sqlite"
   :subname (File. app-config-dir "headshotbox.sqlite")})

(defn exec-sql-file [file]
  (let [queries (str/split (slurp (resource file)) #";\r?\n")]
    (apply jdbc/db-do-commands db true queries)))

(defn init-db []
  (exec-sql-file "sql/create.sql"))

(defn wipe-demos []
  (jdbc/with-db-transaction
    [trans db]
    (jdbc/execute! db ["DELETE FROM playerdemos"])
    (jdbc/execute! db ["DELETE FROM demos"])))

(defn init-db-if-absent []
  (if-not (file-exists? (str app-config-dir "/headshotbox.sqlite"))
    (init-db)))

(defn get-config []
  (json/read-str (:value (first (jdbc/query db ["SELECT value FROM meta WHERE key=?" "config"]))) :key-fn keyword))

(defn set-config [dict]
  (jdbc/with-db-transaction
    [trans db]
    (jdbc/execute! db ["UPDATE meta SET value=? WHERE key=?" (json/write-str dict) "config"])))

(defn update-config [dict]
  (set-config (merge (get-config) dict)))

(defn get-steam-api-key [] (:steam_api_key (get-config)))

(defn get-demo-directory [] (:demo_directory (get-config)))

(defn demo-path [demoid]
  (.getPath (io/file (get-demo-directory) demoid)))

(defn kw-steamids-to-long [path dict]
  (assoc-in dict path (into {} (for [[k v] (get-in dict path)] [(Long/parseLong (name k)) v]))))

(defn db-json-to-dict [rows]
  (->> rows
       (map #(assoc % :data (json/read-str (:data %) :key-fn keyword)))
       (map (partial kw-steamids-to-long [:data :players]))))

(defn get-data-version [demoid]
  (:data_version (first (jdbc/query db ["SELECT data_version FROM demos WHERE demoid=?" demoid]))))

(defn demoid-in-db? [demoid]
  "Returns true if the demo is present and is parsed by the latest version"
  (= (get-data-version demoid) latest-data-version))

(defn add-demo [demoid timestamp map steamids data]
  (jdbc/with-db-transaction
    [trans db]
    (let [data-version (get-data-version demoid)]
      (cond
        (nil? data-version)
        (do
          (debug "Adding demo data for" demoid)
          (jdbc/execute! db ["INSERT INTO demos (demoid, timestamp, map, data_version, data) VALUES (?, ?, ?, ?, ?)"
                             demoid timestamp map latest-data-version (json/write-str data)])
          (doseq [steamid steamids]
            (jdbc/execute! db ["INSERT INTO playerdemos (demoid, steamid) VALUES (?, ?)"
                               demoid steamid])))
        (not= latest-data-version data-version)
        (do
          (debug "Updating data for demo" demoid)
          (jdbc/execute! db ["UPDATE demos SET data=?, data_version=? WHERE demoid=?"
                             (json/write-str data) latest-data-version demoid]))))))

(defn del-demo [demoid]
  (jdbc/with-db-transaction
    [trans db]
    (jdbc/execute! db ["DELETE FROM playerdemos WHERE demoid=?" demoid])
    (jdbc/execute! db ["DELETE FROM demos WHERE demoid=?" demoid])))

(defn keep-only [demoids]
  (if (count demoids)
    (jdbc/with-db-transaction
      [trans db]
      (let [demoids-str (str/join ", " (map #(str "\"" % "\"") demoids))]
        (jdbc/execute! db [(str "DELETE FROM playerdemos WHERE demoid NOT IN (" demoids-str ")")])
        (jdbc/execute! db [(str "DELETE FROM demos WHERE demoid NOT IN (" demoids-str ")")])))))

(defn get-all-demos []
  (->>
    (jdbc/query db [(str "SELECT demos.demoid, data FROM demos")])
    (db-json-to-dict)
    (map #(assoc (:data %) :demoid (:demoid %)))))

(defn get-steamid-info [steamids]
  (->>
    (jdbc/query db [(str "SELECT steamid, timestamp, data FROM steamids WHERE steamid IN (" (str/join ", " steamids) ")")])
    (map #(assoc % :data (json/read-str (:data %) :key-fn keyword)))
    (map #(assoc (:data %) :steamid (:steamid %) :timestamp (:timestamp %)))))

(defn update-steamids [steamids-info]
  (jdbc/with-db-transaction
    [trans db]
    (do
      (jdbc/execute! db [(str "DELETE FROM steamids WHERE steamid IN (" (str/join ", " (keys steamids-info)) ")")])
      (doseq [steamid-info steamids-info]
        (jdbc/execute! db ["INSERT INTO steamids (steamid, timestamp, data) VALUES (?, ?, ?)"
                           (first steamid-info) (current-timestamp) (json/write-str (second steamid-info))]))
      ))
  steamids-info)

(defn get-demo-notes [demoid]
  (jdbc/with-db-transaction
    [trans db]
    (:notes (first (jdbc/query db ["SELECT notes FROM demos WHERE demoid=?" demoid])))))

(defn set-demo-notes [demoid notes]
  (jdbc/with-db-transaction
    [trans db]
    (jdbc/execute! db ["UPDATE demos SET notes=? WHERE demoid=?" notes demoid])))