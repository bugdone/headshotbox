(ns hsbox.version)
(taoensso.timbre/refer-timbre)

(defmacro get-version []
  (System/getProperty "hsbox.version"))

(def os-name
  (let [name (clojure.string/lower-case (System/getProperty "os.name"))]
    (cond
      (.contains name "nux") "linux"
      (.contains name "win") "windows"
      :else nil)))

(def latest-version (atom (get-version)))

(defn get-latest-version []
  (let [url (if (= os-name "linux")
              "https://raw.githubusercontent.com/bugdone/headshotbox/master/latest-linux-version"
              "https://raw.githubusercontent.com/bugdone/headshotbox/master/latest-windows-version")]
    (try
      (info "Checking latest version on" url)
      (swap! latest-version (fn [_] (slurp url)))
      (debug "Latest version" @latest-version)
      (catch Exception e (error "Cannot get latest version info" e)))))

(defn update-latest-version-every-day []
  (while true
    (get-latest-version)
    (Thread/sleep (* 1000 3600 24))))
