(ns hsbox.core
  (:require [hsbox.handler :refer [create-app set-openid-settings]]
            [hsbox.db :as db]
            [hsbox.version :as version]
            [hsbox.indexer :as indexer]
            [hsbox.demo :refer [set-demoinfo-dir]]
            [hsbox.stats :as stats]
            [ring.adapter.jetty]
            [clojure.string :as string]
            [clojure.tools.cli :as cli]
            [taoensso.timbre :as timbre]
            [taoensso.timbre.appenders.core :as appenders])
  (:import (java.io File))
  (:import (java.net BindException URI)
           (hsbox.java SysTrayIcon))
  (:gen-class))

(timbre/refer-timbre)

(def cli-options
  [[nil "--port PORT" "Port number"
    :default 4000
    :parse-fn #(Integer/parseInt %)
    :validate [#(< 0 % 0x10000) "Must be a number between 1 and 65535"]]
   [nil "--portable" "Uses current directory for .sqlite and .log files"]
   [nil "--no-indexer" "Does not parse new demos from the demo directory"]
   [nil "--steamapi-cache days" "Days to cache steam info"
    :default 1
    :parse-fn #(Integer/parseInt %)]
   [nil "--admin-steamid steamid64" "Changing settings and adding notes requires logging in with this steamid64"]
   [nil "--openid-realm url" "Realm url used by OpenID"]
   [nil "--demoinfo-dir directory" "Directory where demoinfogo is located (default is current dir)"]
   [nil "--systray" "Add icon to systray if systray is supported on the current platform"]
   ["-h" "--help"]])


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
          run-indexer? (not (:no-indexer options))
          demoinfo-dir (:demoinfo-dir options)]
      (cond (:help options) (exit 0 summary)
            errors (exit 1 (str summary (error-msg errors))))

      (when portable?
        (db/set-portable))
      (when (not-empty demoinfo-dir)
        (set-demoinfo-dir demoinfo-dir))
      (let [log-file (.getCanonicalFile (File. db/app-config-dir "headshotbox.log"))]
        (timbre/merge-config!
          {:appenders {:spit (appenders/spit-appender {:fname log-file})}})
        (when (:systray options)
          (let [uri (if (:openid-realm options)
                      (URI. (:openid-realm options))
                      (URI. (str "http://localhost:" (:port options))))]
            (SysTrayIcon. uri log-file))))
      (info "HeadshotBox" (version/get-version) (if portable? "portable" "")
            (if (not run-indexer?) "no indexer" ""))
      (future (version/update-latest-version-every-day))

      (set-openid-settings options)
      (.start (ring.adapter.jetty/run-jetty (create-app) {:port (:port options) :join? false}))
      (db/init-db-if-absent)
      (db/upgrade-db)
      (when run-indexer?
        (indexer/set-indexed-path (db/get-demo-directory))
        (indexer/set-indexing-state true)
        (stats/delete-old-demos))
      (stats/init-cache)
      (hsbox.steamapi/init-steam-stale-days (get options :steamapi-cache 1))
      (future (stats/update-players-steam-info))
      (when run-indexer?
        (indexer/add-demo-directory (db/get-demo-directory))
        (indexer/run)))
    (catch BindException e (do
                             (error e)
                             (System/exit 1)))
    (catch Throwable e (error e))))
