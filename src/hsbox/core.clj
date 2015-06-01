(ns hsbox.core
  (:require [hsbox.handler :refer [app]]
            [hsbox.db :as db]
            [hsbox.version :as version]
            [hsbox.indexer :as indexer]
            [hsbox.stats :as stats]
            [ring.adapter.jetty]
            [taoensso.timbre :as timbre])
  (:import (java.io File))
  (:import (java.net BindException))
  (:gen-class))

(timbre/set-config! [:appenders :spit :enabled?] true)
(timbre/set-config! [:shared-appender-config :spit-filename] (File. db/app-config-dir "headshotbox.log"))
(timbre/refer-timbre)

(defn -main [& args]
  (try
    (let [port (try
                 (Integer/parseInt (nth args 0))
                 (catch Exception e 4000))
          flag-present (fn [flag] (some #(= % flag) args))
          portable? (flag-present "-portable")
          run-indexer? (not (flag-present "-noindexer"))
          server (ring.adapter.jetty/run-jetty #'app {:port port :join? false})]
      (info "HeadshotBox" (version/get-version) (if portable? "portable" ""))
      (when portable?
        (db/set-portable))
      (future (version/update-latest-version-every-day))
      (.start server)
      (db/init-db-if-absent)
      (db/upgrade-db)
      (when run-indexer?
        (indexer/set-indexed-path (db/get-demo-directory))
        (indexer/set-indexing-state true)
        (db/keep-only (->> (db/get-demo-directory)
                           (clojure.java.io/as-file)
                           file-seq
                           (map #(.getName %)))))
      (stats/init-cache)
      (when run-indexer?
        (indexer/add-demo-directory (db/get-demo-directory))
        (indexer/run)))
    (catch BindException e (do
                             (error e)
                             (System/exit 1)))
    (catch Throwable e (error e))))