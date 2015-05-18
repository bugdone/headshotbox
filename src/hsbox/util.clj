(ns hsbox.util
  (require [clojure.java.io :refer [as-file]]))

(defn file-exists? [path]
  (let [file (as-file path)]
    (and (.exists file) (not (.isDirectory file)))))

(defn path-exists? [path]
  (let [p (as-file path)]
    (.exists p)))

(defn last-modified [path]
  (/ (.lastModified (as-file path)) 1000))

(defn current-timestamp []
  (quot (System/currentTimeMillis) 1000))
