(ns hsbox.indexer
  (:require [hsbox.demo :refer [get-demo-info get-demo-id]]
            [hsbox.db :as db :refer [demoid-in-db?]]
            [hsbox.stats :as stats]
            [clojure.java.io :refer [as-file]]
            [hsbox.mynotify :as notify :refer [ENTRY_CREATE ENTRY_MODIFY ENTRY_DELETE]]
            [hsbox.util :refer [current-timestamp path-exists? last-modified]]
            [taoensso.timbre :as timbre])
  (:import (java.io File)))

(timbre/refer-timbre)

; Time to wait after the last filesystem event for a file before processing it
(def grace-period 5)
(def paths (atom {}))
(def current-indexed-path (atom nil))
(def indexing-running? (atom true))

(defn del-demo [path]
  (let [demoid (get-demo-id path)]
    (db/del-demo demoid)
    (stats/del-demo demoid)))

; TODO handle overflow
(defn handle-event [path kind]
  (if (contains? (set [ENTRY_CREATE ENTRY_MODIFY]) kind)
    (swap! paths assoc path (current-timestamp))
    (if (= kind ENTRY_DELETE)
      (del-demo path))))

(defn set-indexed-path [path]
  (if (not (path-exists? path))
    (warn "Invalid path" path)
    (try
      (if (not (nil? @current-indexed-path))
        (notify/unregister @current-indexed-path))
      (reset! current-indexed-path path)
      (notify/register path handle-event)
      (catch Throwable e (error e)))))

(defn set-indexing-state [state]
  (reset! indexing-running? state))

(defn is-running? []
  @indexing-running?)

(defn add-demo [path]
  (let [demoid (get-demo-id path)
        mtime (last-modified path)]
    (when-not (demoid-in-db? demoid mtime)
      (debug "Adding path" path)
      (try
        (let [demo-info (get-demo-info path)
              {:keys [timestamp map]} demo-info]
          (db/add-demo demoid timestamp mtime map demo-info)
          (stats/add-demo (merge demo-info {:timestamp timestamp :demoid demoid})))
        (catch Throwable e
          (error "Cannot parse demo" path)
          (error e))))))

(defn add-demo-directory [path]
  (->> (clojure.java.io/as-file path)
       file-seq
       (map #(swap! paths assoc (.getCanonicalPath %) 0))
       dorun))

;(defn rebuild-db []
;  ; TODO: mark all demos with version 0
;  (do
;    (wipe-db)
;    (update-db)))

(defn set-config [config]
  (let [old-demo-dir (db/get-demo-directory)
        demo-dir (:demo_directory config)]
    (db/set-config config)
    (when (not= (as-file old-demo-dir) (as-file demo-dir))
      (db/wipe-demos)
      (stats/init-cache)
      (set-indexed-path demo-dir)
      (add-demo-directory demo-dir))))

(defn run []
  (debug "Indexer started")
  (future (notify/watch))
  (while true
    (doseq [path (map key (filter #(< (+ (val %) grace-period) (current-timestamp)) @paths))]
      (while (not @indexing-running?)
        (Thread/sleep 1000))
      (swap! paths #(dissoc % path))
      (when (second (re-matches #".*\.dem" (get-demo-id path)))
        (add-demo path)))
    (Thread/sleep 5000)))
