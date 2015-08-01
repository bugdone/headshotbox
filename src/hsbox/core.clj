(ns hsbox.core
  (:require [hsbox.handler :refer [app]]
            [hsbox.db :as db]
            [hsbox.version :as version]
            [hsbox.indexer :as indexer]
            [hsbox.stats :as stats]
            [ring.adapter.jetty]
            [clojure.string :as string]
            [clojure.tools.cli :as cli]
            [taoensso.timbre :as timbre])
  (:import (java.io File))
  (:import (java.net BindException))
  (:gen-class))

(timbre/set-config! [:appenders :spit :enabled?] true)
(timbre/set-config! [:shared-appender-config :spit-filename] (File. db/app-config-dir "headshotbox.log"))
(timbre/refer-timbre)

(def cli-options
  [[nil "--port PORT" "Port number"
    :default 4000
    :parse-fn #(Integer/parseInt %)
    :validate [#(< 0 % 0x10000) "Must be a number between 1 and 65535"]]
   [nil "--portable" "Uses current directory for .sqlite and .log files"]
   [nil "--no-indexer" "Does not parse new demos from the demo directory"]
   ["-h" "--help"]
   ])

(defn error-msg [errors]
  (str "The following errors occurred while parsing your command:\n\n"
       (string/join \newline errors)))

(defn exit [status msg]
  (println "HeadshotBox" (version/get-version))
  (println msg)
  (System/exit status))

(defn -main [& args]
  (try
    (let [{:keys [options arguments errors summary]} (cli/parse-opts args cli-options)
          portable? (:portable options)
          run-indexer? (not (:no-indexer options))]
      (cond (:help options) (exit 0 summary)
            errors (exit 1 (error-msg errors)))

      (info "HeadshotBox" (version/get-version) (if portable? "portable" "")
            (if (not run-indexer?) "no indexer" ""))
      (when portable?
        (db/set-portable))
      (future (version/update-latest-version-every-day))
      (.start (ring.adapter.jetty/run-jetty #'app {:port (:port options) :join? false}))
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