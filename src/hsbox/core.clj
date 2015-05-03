(ns hsbox.core
  (:require [hsbox.handler :refer [app]]
            [hsbox.db :as db]
            [hsbox.version :as version]
            [hsbox.indexer :as indexer]
            [hsbox.stats :as stats]
            [ring.adapter.jetty]
            [taoensso.timbre :as timbre])
  (:import (java.io File))
  (:gen-class))

(timbre/set-config! [:appenders :spit :enabled?] true)
(timbre/set-config! [:shared-appender-config :spit-filename] (File. db/app-config-dir "headshotbox.log"))
(timbre/refer-timbre)

(defn -main [& args]
  (try
    (let [port (try
                 (Integer/parseInt (nth args 0))
                 (catch Exception e 4000))
          server (ring.adapter.jetty/run-jetty #'app {:port port :join? false})]
      (info "HeadshotBox " (version/get-version))
      (future (version/update-latest-version-every-day))
      (.start server)
      (db/init-db-if-absent)
      (stats/init-cache)
      (indexer/set-indexed-path (db/get-demo-directory))
      (indexer/set-indexing-state true)
      (db/keep-only (->> (db/get-demo-directory)
                         (clojure.java.io/as-file)
                         file-seq
                         (map #(.getName %))))
      (stats/init-cache)
      (indexer/add-demo-directory (db/get-demo-directory))
    (indexer/run))
    (catch Exception e (error e))))