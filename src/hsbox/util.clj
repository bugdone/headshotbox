(ns hsbox.util)

(defn file-exists? [path]
  (let [file (clojure.java.io/as-file path)]
    (and (.exists file) (not (.isDirectory file)))))

(defn path-exists? [path]
  (let [p (clojure.java.io/as-file path)]
    (.exists p)))

(defn current-timestamp []
  (quot (System/currentTimeMillis) 1000))
