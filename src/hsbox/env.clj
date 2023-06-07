(ns hsbox.env
  (:require [hsbox.util :refer [make-path]]))

(def steam-path (System/getenv "STEAM_PATH"))
(def csgo-path (System/getenv "CSGO_PATH"))
(def hlae-path (System/getenv "HLAE_PATH"))
(def reshade-path (System/getenv "RESHADE_PATH"))
(def output-folder (System/getenv "MOVIE_FOLDER_PATH"))
(def raw-data-folder (make-path output-folder "tmp"))
