(ns hsbox.db
  (:require [clojure.data.json :as json]
            [clojure.java.jdbc :as jdbc]
            [clojure.java.io :as io :refer [resource]]
            [clojure.string :as str]
            [hsbox.util :refer [current-timestamp file-exists? get-canonical-path]]
            [clojure.java.io :refer [as-file]]
            [taoensso.timbre :as timbre])
  (:import (java.io File)))

(timbre/refer-timbre)
(def latest-data-version {"valve"  4
                          "esea"   5
                          "faceit" 4
                          "cevo"   4})

(def schema-version 6)
;(set! *warn-on-reflection* true)

(def app-config-dir
  (let [config-home (if-let [xdg (System/getenv "XDG_CONFIG_HOME")]
                      (File. xdg)
                      (File. (System/getProperty "user.home") ".config"))
        app-config (File. config-home "headshotbox")]
    (.mkdir config-home)
    (.mkdir app-config)
    app-config))

(def db nil)
(def db-lock (Object.))
(def config (atom nil))

(defmacro with-db-transaction [t-con & body]
  `(locking db-lock
     (jdbc/with-db-transaction [~t-con db]
                               ~@body)))

(defn query-db [arg]
  (with-db-transaction t-con (jdbc/query t-con arg)))

(defn set-portable []
  (def app-config-dir (File. ".")))

(defn exec-sql-file [t-con file]
  (let [queries (str/split (slurp (resource file)) #";\r?\n")]
    (apply jdbc/db-do-commands t-con queries)))

(defn init-db []
  (with-db-transaction t-con
                       (exec-sql-file t-con "sql/create.sql")))

(defn wipe-demos []
  (with-db-transaction t-con
                       (jdbc/execute! t-con ["DELETE FROM demos"])))

(defn init-db-if-absent []
  (def hsbox.db/db {:classname   "org.sqlite.JDBC"
                    :subprotocol "sqlite"
                    :subname     (File. app-config-dir "headshotbox.sqlite")})
  (if-not (file-exists? (str app-config-dir "/headshotbox.sqlite"))
    (init-db)))

(defn get-meta-value [key]
  (json/read-str (:value (first (query-db ["SELECT value FROM meta WHERE key=?" key]))) :key-fn keyword))

(defn get-current-schema-version [] (get-meta-value "schema_version"))

(defn get-config []
  (when (nil? @config)
    (reset! config (get-meta-value "config")))
  @config)

(defn get-demo-directory [] (:demo_directory (get-config)))

(defn half-parsed-demo? [{:keys [score rounds players]}]
  (let [score1 (first (:score score))
        score2 (second (:score score))]
    (or (= 0 (count players))
        (not= 2 (count (:score score)))
        (and (not (:surrendered score)) (not= score1 score2 15) (< score1 16) (< score2 16))
        (not (empty? (filter #(not (:tick_end %)) rounds))))))

(defn kw-steamids-to-long [path dict]
  (assoc-in dict path (into {} (for [[k v] (get-in dict path)] [(Long/parseLong (name k)) v]))))

(defn db-json-to-dict [rows]
  (letfn [(round-players-to-long [rounds]
            (->> rounds
                 (map (partial kw-steamids-to-long [:players]))
                 (map (partial kw-steamids-to-long [:disconnected]))
                 (map (partial kw-steamids-to-long [:damage]))))]
    (->> rows
         (map #(assoc % :data (json/read-str (:data %) :key-fn keyword)))
         (map (partial kw-steamids-to-long [:data :players]))
         (map (partial kw-steamids-to-long [:data :mm_rank_update]))
         (map (partial kw-steamids-to-long [:data :player_slots]))
         (map #(assoc-in % [:data :rounds] (round-players-to-long (get-in % [:data :rounds])))))))

(defn get-relative-path [demo-str-path]
  "Get the relative path of demo-str-path to demo dir"
  (let [to-path (fn [str-path] (.toAbsolutePath (.toPath (as-file str-path))))]
    (.relativize (to-path (get-demo-directory)) (to-path demo-str-path))))

(defn get-folder [path]
  (let [relative (.getParent (get-relative-path path))]
    (if (nil? relative)
      nil
      (.toString relative))))

(defn get-all-demos []
  (->>
    (query-db [(str "SELECT rowid, path, type, data_version, data FROM demos")])
    (filter #(= (latest-data-version (:type %)) (:data_version %)))
    (db-json-to-dict)
    (map #(assoc (:data %) :demoid (:rowid %)
                           :path (:path %)
                           :folder (get-folder (:path %))))))

(defn sql-demo-paths [paths]
  (str " (" (str/join ", " (map #(str "\"" % "\"") paths)) ")"))

(defn get-all-demos-v1 [t-con]
  (->>
    (jdbc/query t-con ["SELECT demos.demoid, data FROM demos"])
    (db-json-to-dict)
    (map #(assoc (:data %) :demoid (:demoid %)))))

(defn migrate-2 [t-con]
  (exec-sql-file t-con "sql/migrate_1_to_2.sql")
  (let [half-parsed-demos (filter half-parsed-demo? (get-all-demos-v1 t-con))
        demoids (map #(:demoid %) half-parsed-demos)]
    (if (not (empty? demoids))
      (jdbc/execute! t-con [(str "UPDATE demos SET mtime = 0 WHERE demoid IN " (sql-demo-paths demoids))]))))

; So some systems have subsecond precision...
; and for these systems, mtime was a clojure ratio serialized as string
(defn migrate-3 [t-con]
  (let [demos (jdbc/query t-con ["SELECT demos.demoid, mtime FROM demos"])]
    (doseq [demo demos]
      (let [mtime (int (read-string (str (:mtime demo))))]
        (jdbc/execute! t-con ["UPDATE demos SET mtime = ? WHERE demoid = ?" mtime (:demoid demo)])))))

(defn migrate-4 [t-con]
  (exec-sql-file t-con "sql/migrate_3_to_4.sql"))

(defn migrate-5 [t-con]
  ; Rename demoid column to path
  (jdbc/execute! t-con [(str "CREATE TABLE demos_new ("
                             "path TEXT(256) UNIQUE,"
                             "mtime INT,"
                             "timestamp INT,"
                             "type VARCHAR(20),"
                             "map VARCHAR(20) NOT NULL,"
                             "data_version INT,"
                             "data TEXT NOT NULL,"
                             "notes TEXT)")])
  (jdbc/execute! t-con [(str "INSERT INTO demos_new(path, mtime, timestamp, type, map, data_version, data, notes) "
                             "SELECT demoid, mtime, timestamp, type, map, data_version, data, notes FROM demos")])
  (jdbc/execute! t-con ["DROP TABLE demos"])
  (jdbc/execute! t-con ["ALTER TABLE demos_new RENAME TO demos"])
  ; Fill in path relative to demo dir
  (let [demo-dir (as-file (get-demo-directory))]
    (->>
      demo-dir
      file-seq
      (map #(jdbc/execute! t-con ["UPDATE demos SET path = ? WHERE path = ?"
                                  (get-relative-path %)
                                  (.getName %)]))
      dorun)))

(defn migrate-6 [t-con]
  (let [paths (doall (map #(vector (:rowid %) (:path %))
                          (jdbc/query t-con ["SELECT rowid, path FROM demos"])))]
    (doseq [[rowid str-path] paths]
      (let [demo-path (.toPath (as-file str-path))
            demos (->>
                    (get-demo-directory)
                    (as-file)
                    (file-seq)
                    (map #(.toPath %)))
            update-path (fn [path]
                          (jdbc/execute! t-con ["UPDATE demos SET path = ? WHERE rowid = ?"
                                                (.getCanonicalPath (.toFile path)) rowid])
                          (debug "updating for rowid" rowid "path" (.getCanonicalPath (.toFile path))))]
        (if (.isAbsolute demo-path)
          (let [same-name (filter #(= (.getFileName %) (.getFileName demo-path)) demos)]
            (debug "isAbsolute" demo-path "count" (count same-name))
            (if (= 1 (count same-name))
              (update-path (first same-name))))
          (do
            (debug "is relative" demo-path)
            (update-path (.toPath (File. (as-file (get-demo-directory)) str-path)))))))))

(def migrations {1 [2 migrate-2]
                 2 [3 migrate-3]
                 3 [4 migrate-4]
                 4 [5 migrate-5]
                 5 [6 migrate-6]})

(defn get-migration-plan []
  (loop [plan []
         version (get-current-schema-version)]
    (cond
      (= version schema-version) plan
      (not (contains? migrations version)) (throw (Exception. "Cannot find a migration plan"))
      :else (let [[next-version f] (get migrations version)]
              (recur (conj plan [next-version f]) next-version)))))

(defn upgrade-db []
  (let [migration-plan (get-migration-plan)]
    (doall (map #(let [version (first %) procedure (second %)]
                  (warn "Migrating from schema version" (get-current-schema-version) "to" version)
                  (with-db-transaction t-con
                                       (procedure t-con)
                                       (jdbc/execute! t-con ["UPDATE meta SET value = ? WHERE key = ?" version "schema_version"])))
                migration-plan))))

(defn set-config [dict]
  (with-db-transaction t-con
                       (jdbc/execute! t-con ["UPDATE meta SET value=? WHERE key=?" (json/write-str dict) "config"]))
  (reset! config dict))

(defn update-config [dict]
  (set-config (merge (get-config) dict)))

(defn get-steam-api-key [] (:steam_api_key (get-config)))

(defn demoid-present? [demoid]
  (first (query-db ["SELECT rowid FROM demos WHERE rowid=?" demoid])))

(defn- get-data-version [demo-path]
  (first (query-db ["SELECT type, data_version FROM demos WHERE path=?" demo-path])))

(defn- get-demo-mtime [demo-path]
  (:mtime (first (query-db ["SELECT mtime FROM demos WHERE path=?" demo-path]))))

(defn demo-path-in-db? [demo-path mtime]
  "Returns true if the demo is present, was parsed by the latest version at/after mtime"
  (let [canonical-path (get-canonical-path demo-path)
        mtime-db (get-demo-mtime canonical-path)]
    (and
      (not (nil? mtime-db))
      (<= mtime mtime-db)
      (let [{type :type data-version :data_version} (get-data-version canonical-path)]
        (if (not (nil? type))
          (= (get latest-data-version type) data-version)
          true)))))

(defn- get-demo-id [demo-path t-con]
  (:rowid (first (jdbc/query t-con ["SELECT rowid FROM demos WHERE path=?" demo-path]))))

(defn del-demo [abs-path]
  (let [abs-path (get-canonical-path abs-path)]
    (with-db-transaction t-con
                         (let [demoid (get-demo-id abs-path t-con)]
                           (jdbc/execute! t-con ["DELETE FROM demos WHERE rowid=?" demoid])
                           demoid))))

(defn add-demo [demo-path mtime data]
  (let [demo-path (get-canonical-path demo-path)
        {:keys [timestamp map type]} data
        data-str (json/write-str data)
        data-version (get latest-data-version type)]
    (assert (not (and (nil? type) (nil? timestamp) (nil? map))))
    (with-db-transaction t-con
                         (if (get-demo-id demo-path t-con)
                           (do
                             (debug "Updating data for demo" demo-path)
                             (jdbc/execute! t-con ["UPDATE demos SET data=?, data_version=?, timestamp=?, mtime=?, map=?, type=? WHERE path=?"
                                                   data-str data-version timestamp mtime map type demo-path]))
                           (do
                             (debug "Adding demo data for" demo-path)
                             (jdbc/execute! t-con ["INSERT INTO demos (path, timestamp, mtime, map, data_version, data, type) VALUES (?, ?, ?, ?, ?, ?, ?)"
                                                   demo-path timestamp mtime map data-version data-str type])))
                         (get-demo-id demo-path t-con))))

(defn keep-only [paths]
  (if (count paths)
    (with-db-transaction t-con
                         (jdbc/execute! t-con [(str "DELETE FROM demos WHERE path NOT IN " (sql-demo-paths paths))]))))

(defn get-steamid-info [steamids]
  (->>
    (query-db [(str "SELECT steamid, timestamp, data FROM steamids WHERE steamid IN (" (str/join ", " steamids) ")")])
    (map #(assoc % :data (json/read-str (:data %) :key-fn keyword)))
    (map #(assoc (:data %) :steamid (:steamid %) :timestamp (:timestamp %)))))

(defn update-steamids [steamids-info]
  (with-db-transaction t-con
                       (do
                         (jdbc/execute! t-con [(str "DELETE FROM steamids WHERE steamid IN (" (str/join ", " (keys steamids-info)) ")")])
                         (doseq [steamid-info steamids-info]
                           (jdbc/execute! t-con ["INSERT INTO steamids (steamid, timestamp, data) VALUES (?, ?, ?)"
                                                 (first steamid-info) (current-timestamp) (json/write-str (second steamid-info))]))))
  steamids-info)

(defn get-demo-notes [demoid]
  (:notes (first (query-db ["SELECT notes FROM demos WHERE rowid=?" demoid]))))

(defn set-demo-notes [demoid notes]
  (with-db-transaction t-con
                       (jdbc/execute! t-con ["UPDATE demos SET notes=? WHERE rowid=?" notes demoid])))