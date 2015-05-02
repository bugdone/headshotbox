(ns hsbox.mynotify
  (:import (java.nio.file Paths FileSystems))
  (:require [taoensso.timbre :as timbre]))

(timbre/refer-timbre)

(def ENTRY_CREATE java.nio.file.StandardWatchEventKinds/ENTRY_CREATE)
(def ENTRY_DELETE java.nio.file.StandardWatchEventKinds/ENTRY_DELETE)
(def ENTRY_MODIFY java.nio.file.StandardWatchEventKinds/ENTRY_MODIFY)

(def watcher (.. FileSystems getDefault newWatchService))
(def watch-keys (atom {}))

(defrecord KeyInfo [path callback])

(defn get-Path [path]
  (Paths/get path (make-array String 0)))

(defn register [path callback]
  (let [dir (get-Path path)
        key (.register dir watcher (into-array [ENTRY_CREATE ENTRY_DELETE ENTRY_MODIFY]))]
    (swap! watch-keys assoc key (->KeyInfo dir callback))))

(defn unregister [path]
  (swap! watch-keys (fn [k]
                      (let [entry (first (filter #(= (:path (val %)) (get-Path path)) k))]
                        (if (nil? entry)
                          k
                          (dissoc k (key entry)))))))

(defn watch []
  (while true
    (do
      (let [key (. watcher take)
            info (get @watch-keys key)]
        (if (not (nil? info))
          (doseq [event (.pollEvents key)]
            (let [kind (.. event kind)
                  path (->> event
                            .context
                            (.resolve (:path info))
                            str)]
              ((:callback info) path kind))))
        (when (not (.reset key))
          (warn "Not watching path anymore" (:path info))
          (unregister (str (:path info))))))))
