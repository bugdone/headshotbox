(ns hsbox.util)

(defn file-exists? [path]
  (let [file (clojure.java.io/as-file path)]
    (and (.exists file) (not (.isDirectory file)))))


(defn current-timestamp []
  (quot (System/currentTimeMillis) 1000))
