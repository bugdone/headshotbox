(ns hsbox.movie
  (:require [clojure.java.io :as io]
            [clojure.stacktrace :refer [print-cause-trace]]
            [clojure.string :as str]
            [hsbox.launch :refer [escape-path kill-csgo-process write-vdm-file]]
            [hsbox.util :refer [make-path]]
            [hsbox.env :as env]
            [hsbox.stats :as stats]
            [taoensso.timbre :as timbre]))

(timbre/refer-timbre)

(def resolution [1920 1080])

(defn delete-directory-recursive [^java.io.File file]
  (when (.isDirectory file)
    (run! delete-directory-recursive (.listFiles file)))
  (io/delete-file file))

(defn record-round [demoid steamid round-number async? & {:keys [kill-filter] :or {kill-filter (fn [_] true)}}]
  (println "record-round" demoid steamid round-number)
  (try
    (kill-csgo-process)
    (let [demo (get stats/demos demoid)
          _ (assert (:path demo))
          _ (assert (:demoid demo))
          path (clojure.string/replace (:path demo) "\\" "/")
          demo (assoc demo :path path)
          vdm-info (write-vdm-file demo steamid 0 round-number "round" :kill-filter kill-filter)
          process-clip (fn [index clip-name]
                         (let [directory (str env/raw-data-folder "\\" (format "take%04d" index))
                               output-path (make-path env/output-folder (str clip-name ".mp4"))
                               args (concat [(make-path env/hlae-path "ffmpeg" "bin" "ffmpeg.exe")]
                                            ["-i" (escape-path (str directory "\\HSBOX\\video.mp4"))
                                             "-i" (escape-path (str directory "\\audio.wav"))]
                                            (str/split "-y -c:v copy -c:a aac" #" ")
                                            [(escape-path output-path)])]
                           _ (println args)
                           (apply clojure.java.shell/sh args)
                           (delete-directory-recursive (io/file directory))
                           output-path))
          process-videos (fn [clips-paths]
                           (let [ffmpeg-video-list (str env/raw-data-folder "\\clips.txt")
                                 args (concat [(make-path env/hlae-path "ffmpeg" "bin" "ffmpeg.exe")]
                                              ["-y" "-safe" "0" "-f" "concat" "-i" ffmpeg-video-list "-c" "copy"
                                               (escape-path (make-path env/output-folder (str demoid "_" steamid "_" round-number ".mp4")))])
                                 _ (println args)]
                             (spit ffmpeg-video-list (str/join "\r\n" (map #(str "file " (clojure.string/replace % "\\" "/")) clips-paths)))
                             (apply clojure.java.shell/sh args)
                             (io/delete-file (io/file ffmpeg-video-list))
                             (doall (map #(io/delete-file (io/file %)) clips-paths))))
          record-fn (fn []
                      (future
                        (let [prefix-args [(make-path env/hlae-path "HLAE.exe") "-csgoLauncher" "-noGui" "-autoStart"
                                           "-gfxEnabled" "true" "-gfxWidth" (str (first resolution)) "-gfxHeight" (str (second resolution)) "-gfxFull" "true"
                                           "-programPath" env/csgo-path
                                           "-customLoader" "-hookDllPath" (make-path env/hlae-path "AfxHookSource.dll")]
                              reshade-args (if (nil? env/reshade-path)
                                             []
                                             ["-hookDllPath" (make-path env/reshade-path "ReShade32.dll")])
                              suffix-args ["-cmdLine" (str "\"" "-w " (str (first resolution))
                                                           " -h " (str (second resolution))
                                                           " -insecure -windowed -novid +playdemo "
                                                           path "@" (:tick vdm-info) "\"")]
                              args (concat prefix-args reshade-args suffix-args)]
                          (println args)
                          (apply clojure.java.shell/sh args))
                        (process-videos (doall (map-indexed #(process-clip %1 %2) (:clip-ids vdm-info))))))]
      (if async? (record-fn)
                 @(record-fn)))
    (catch Throwable e (do
                         (print-cause-trace e)
                         (error e)))))
(defn make-movie [steamid plays from filters]
  (let [big-plays (drop from (stats/get-big-plays steamid plays filters))]
    (doall (map #(record-round (:demoid %) steamid (:round-number %) false) big-plays))))