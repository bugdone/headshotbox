(ns hsbox.util
  (require [clojure.java.io :refer [as-file]]))

(defn file-exists? [path]
  (let [file (as-file path)]
    (and (.exists file) (not (.isDirectory file)))))

(defn path-exists? [path]
  (let [p (as-file path)]
    (and p (.exists p))))

(defn is-dir? [path]
  (.isDirectory (as-file path)))

(defn is-demo? [path]
  (and (.isFile (as-file path)) (.endsWith path ".dem")))

(defn last-modified [path]
  (int (/ (.lastModified (as-file path)) 1000)))

(defn current-timestamp []
  (quot (System/currentTimeMillis) 1000))

(defn file-name [path]
  (.getName (clojure.java.io/as-file path)))

(defn get-canonical-path [path]
  (.getCanonicalPath (as-file path)))