(ns hsbox.indexer
  (:require [hsbox.demo :refer [get-demo-info]]
            [hsbox.db :as db :refer [demo-path-in-db?]]
            [hsbox.stats :as stats]
            [clojure.java.io :refer [as-file]]
            [hsbox.mynotify :as notify :refer [ENTRY_CREATE ENTRY_MODIFY ENTRY_DELETE]]
            [hsbox.util :refer [current-timestamp path-exists? last-modified is-dir? is-demo? get-canonical-path]]
            [taoensso.timbre :as timbre])
  (:import (java.util.concurrent.locks ReentrantLock)
           (java.util.concurrent TimeUnit)))

(timbre/refer-timbre)

; Time to wait after the last filesystem event for a file before processing it
(def grace-period 5)
(def paths (atom {}))
(def current-indexed-path (atom nil))
(def indexing-running? (atom true))
(def parsing? (atom false))

(defn del-demo [path]
  (let [demoid (db/del-demo path)]
    (stats/del-demo demoid)))

(defn- handle-file-event [path kind]
  (let [path (get-canonical-path path)]
    (cond
      (contains? (set [ENTRY_CREATE ENTRY_MODIFY]) kind) (swap! paths assoc path (current-timestamp))
      (= kind ENTRY_DELETE) (del-demo path))))

(declare handle-event)

(defn- handle-dir-event [path kind]
  (cond
    (= kind ENTRY_CREATE) (notify/register path handle-event)
    (= kind ENTRY_DELETE) (notify/unregister path)))

; TODO handle overflow
(defn handle-event [path kind]
  (cond
    (.endsWith path ".dem") (handle-file-event path kind)
    (is-dir? path) (handle-dir-event path kind)))

(defn- for-all-subpaths [path f]
  (->> (as-file path)
       file-seq
       (map #(f (.getCanonicalPath %)))
       dorun))

(defn set-indexed-path [path]
  (if (not (path-exists? path))
    (warn "Invalid path" path)
    (try
      (notify/unregister-all)
      (reset! current-indexed-path path)
      (for-all-subpaths path #(if (is-dir? %) (notify/register % handle-event)))
      (notify/register path handle-event)
      (catch Throwable e (error e)))))

(defn set-indexing-state [state]
  (reset! indexing-running? state))

(defn is-running? []
  @indexing-running?)

(defn is-parsing? []
  (and @parsing? @indexing-running?))

(defn demos-left []
  (count @paths))

(defn add-demo [abs-path]
  (try
    (let [mtime (last-modified abs-path)]
      (when-not (demo-path-in-db? abs-path mtime)
        (debug "Adding path" abs-path)
        (try
          (let [demo-info (get-demo-info abs-path)]
            (if (or (empty? (:rounds demo-info)) (empty? (:players demo-info)))
              (throw (Exception. (str "Demo" abs-path "has" (count (:rounds demo-info)) "rounds and"
                                      (count (:players demo-info)) "players"))))
            (let [demoid (db/add-demo abs-path mtime demo-info)]
              (stats/add-demo (assoc demo-info :demoid demoid
                                               :path abs-path
                                               :folder (hsbox.db/get-folder abs-path)))))
          (catch Throwable e
            (error "Cannot parse demo" abs-path)
            (error e)))))
    (catch Throwable e
      (error e))))

(defn add-demo-directory [path]
  (for-all-subpaths path #(if (is-demo? %) (swap! paths assoc % 0))))

;(defn rebuild-db []
;  ; TODO: mark all demos with version 0
;  (do
;    (wipe-db)
;    (update-db)))

(def directory-scan-lock (ReentrantLock.))
(def directory-scan-cond (.newCondition directory-scan-lock))
(def directory-scan-interval (atom 0))

(defn set-config [config]
  (let [old-demo-dir (db/get-demo-directory)
        demo-dir (:demo_directory config)]
    (db/set-config config)
    (reset! directory-scan-interval (get config :directory_scan_interval 0))
    (try
      (.lock directory-scan-lock)
      (.signal directory-scan-cond)
      (finally
        (.unlock directory-scan-lock)))
    (when (not= (get-canonical-path old-demo-dir) (get-canonical-path demo-dir))
      (db/wipe-demos)
      (stats/init-cache)
      (set-indexed-path demo-dir)
      (add-demo-directory demo-dir))))

(defn scan-demo-directory []
  (def hsbox.indexer/directory-scan-interval (atom (get (db/get-config) :directory_scan_interval 0)))
  (while true
    (try
      (.lock directory-scan-lock)
      (if (zero? @directory-scan-interval)
        (.await directory-scan-cond)
        (.await directory-scan-cond @directory-scan-interval TimeUnit/MINUTES))
      (when (and (not (zero? @directory-scan-interval)) (is-running?))
        (stats/delete-old-demos)
        ; This should delete the old demos from the cache as well
        ; But I'm too lazy
        (add-demo-directory @current-indexed-path))
      (finally
        (.unlock directory-scan-lock)))))

(defn run []
  (debug "Indexer started")
  (future (notify/watch))
  (future (scan-demo-directory))
  (while true
    (doseq [path (map key (filter #(< (+ (val %) grace-period) (current-timestamp)) @paths))]
      (while (not @indexing-running?)
        (Thread/sleep 1000))
      (reset! parsing? true)
      (swap! paths #(dissoc % path))
      (when (is-demo? path)
        (add-demo path)))
    (reset! parsing? false)
    (Thread/sleep 5000)))
