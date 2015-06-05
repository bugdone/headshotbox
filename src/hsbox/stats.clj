(ns hsbox.stats
  (:require [clojure.string :as str]
            [hsbox.db :as db :refer [demo-path get-steam-api-key]]))

(taoensso.timbre/refer-timbre)

(def demos {})
(def player-demos {})

(defn seconds-to-ticks [seconds tickrate]
  (* seconds (/ 1 tickrate)))

(defn enumerate [s]
  (map vector (range) s))

(defn get-player-name-in-demo [steamid demo]
  (get-in demo [:players steamid :name]))

(defn get-players []
  (->> player-demos
       (reduce-kv #(conj % {:steamid %2
                            :demos   (count %3)
                            :name    (get-player-name-in-demo %2 (second (first %3)))}) [])
       (filter #(> (:demos %) 1))
       (sort #(compare (:demos %2) (:demos %)))
       (map #(assoc % :steamid (str (:steamid %))))))

(defn sorted-demos-for-steamid [steamid]
  (sort #(compare (:timestamp %) (:timestamp %2)) (vals (get player-demos steamid))))

(defn get-player-latest-name [steamid]
  (get-player-name-in-demo steamid (first (sorted-demos-for-steamid steamid))))

(defn add-demo [demo]
  (if (not (or (empty? (:rounds demo)) (empty? (:players demo))))
    (do
      ; TODO use atom
      (def hsbox.stats/demos (assoc demos (:demoid demo) demo))
      (doseq [steamid (keys (:players demo))]
        (def hsbox.stats/player-demos (assoc-in player-demos [steamid (:demoid demo)] demo))))
    (warn "Demo" (:demoid demo) "has" (count (:rounds demo)) "rounds and" (count (:players demo)) "players")))

(defn del-demo [demoid]
  (let [demo (get demos demoid)]
    (def hsbox.stats/demos (dissoc demos demoid))
    (doseq [steamid (keys (:players demo))]
      (def hsbox.stats/player-demos (assoc player-demos steamid (dissoc (get player-demos steamid) demoid))))))

(defn weapon-name [^String name]
  (cond
    (or (= name "bayonet") (.startsWith name "knife")) "knife"
    (.startsWith name "usp_silencer") "usp"
    (= name "m4a1") "m4a4"
    (= name "galilar") "galil"
    (= name "molotov_projectile") "inferno"
    (.startsWith name "m4a1_silencer") "m4a1"
    :else name))

(def weapon-names ["mag7" "mp9" "inferno" "elite" "g3sg1" "negev" "fiveseven" "hkp2000" "m4a1" "galil" "xm1014"
                   "ssg08" "nova" "flashbang" "ak47" "m4a4" "tec9" "decoy" "hegrenade" "taser" "famas" "ump45"
                   "mp7" "worldspawn" "cz75a" "glock" "sg556" "mac10" "sawedoff" "p90" "usp" "p250" "scar20"
                   "deagle" "awp" "aug" "world" "m249" "bizon" "knife" "smokegrenade"])

(defn add-hltv-rating [stats]
  "Compute HLTV rating"
  (let [AverageKPR 0.679
        AverageSPR 0.317
        AverageRMK 1.277
        rounds (:rounds stats)
        RoundsWithMultipleKillsRating (/ (apply + (for [x (range 1 6)] (* x x (get-in stats [:rounds_with_kills x]))))
                                         rounds
                                         AverageRMK)
        KillRating (/ (:kills stats) rounds AverageKPR)
        SurvivalRating (/ (- rounds (:deaths stats)) rounds AverageSPR)]
    (assoc stats :rating (/ (+ KillRating
                               (* 0.7 SurvivalRating)
                               RoundsWithMultipleKillsRating)
                            2.7))))

(defn inc-stat-maybe [stats stat value]
  (if value
    (let [stat (if (sequential? stat) stat [stat])]
      (assoc-in stats stat (inc (get-in stats stat 0))))
    stats))

(defn update-stats-with-death [stats {:keys [attacker victim assister weapon headshot]}]
  (let [steamid (:steamid stats)
        weapon (weapon-name weapon)
        enemies? (not= (get (:players stats) steamid) (get (:players stats) victim))
        killed? (and (= steamid attacker) enemies?)
        init-hs-if-needed (fn [stats weapon]
                            (if (and killed? (nil? (get-in stats [:weapons weapon :hs])))
                              (assoc-in stats [:weapons weapon :hs] 0)
                              stats))]
    (-> stats
        (inc-stat-maybe :kills killed?)
        (inc-stat-maybe :kills-this-round killed?)
        (inc-stat-maybe :deaths (= steamid victim))
        (inc-stat-maybe :assists (and (= steamid assister) enemies?))
        (inc-stat-maybe [:weapons weapon :kills] killed?)
        (init-hs-if-needed weapon)
        (inc-stat-maybe :hs (and headshot killed?))
        (inc-stat-maybe [:weapons weapon :hs] (and headshot killed?)))))

(defn team-number [steamid round]
  (get (:players round) steamid 0))

(defn build-clutch-round-fn [enemies exact-enemies? won?]
  (fn [round steamid demo]
    (if (nil? (:tick_end round))
      ; TODO make this check somewhere more sane (eg. when it's read from / written in db)
      (do
        (debug "No tick_end for round" (:number round) "in demo" (:demoid demo))
        false)
      (let [deaths (filter #(<= (:tick %) (:tick_end round)) (:deaths round))
            player-team (team-number steamid round)
            player-death (split-with #(not= (:victim %) steamid) deaths)
            same-team (fn [death] (= player-team (team-number (:victim death) round)))
            not-same-team (comp not same-team)
            last-teammate-death (split-with not-same-team
                                            (reverse (first player-death)))
            dead-teammates (count (filter same-team (second last-teammate-death)))
            alive-enemies (- 5 (count (filter not-same-team (second last-teammate-death))))]
        (and ((if exact-enemies? = >=) alive-enemies enemies) (= 4 dead-teammates) (if won? (= (:winner round) player-team) true))))))

(defn update-stats-with-round [stats round]
  (let [updated-stats (reduce update-stats-with-death
                              (assoc stats :kills-this-round 0 :players (:players round))
                              (:deaths round))
        multikills (:kills-this-round updated-stats)
        ; Super lame hax horrible code the wurst (somewhat better now)
        demo {:demoid (:demoid stats)}]
    (-> updated-stats
        (dissoc :kills-this-round :players)
        (update-in [:rounds_with_kills multikills] inc)
        (inc-stat-maybe :1v1_attempted ((build-clutch-round-fn 1 true false) round (:steamid stats) demo))
        (inc-stat-maybe :1v1_won ((build-clutch-round-fn 1 true true) round (:steamid stats) demo))))
  )

(defn demo-outcome [demo steamid]
  (cond
    (= (:winner demo) 0) :tied
    (= (:winner demo) (team-number steamid (first (:rounds demo)))) :won
    true :lost))

(defn add-stat [stats stat value]
  (assoc stats stat (+ (stats stat) value)))

(defn add-round-numbers [rounds]
  (map #(assoc (second %) :number (+ 1 (first %))) (enumerate rounds)))

(defn update-stats-with-demo [stats demo]
  (-> (reduce update-stats-with-round (assoc stats :demoid (:demoid demo)) (add-round-numbers (:rounds demo)))
      (add-stat (demo-outcome demo (:steamid stats)) 1)
      (add-stat :rounds (count (:rounds demo)))
      (dissoc :demoid)
      (add-hltv-rating)))

(defn initial-stats [steamid]
  {:steamid           steamid
   :kills             0
   :deaths            0
   :assists           0
   :rounds            0
   :won               0
   :lost              0
   :tied              0
   :hs                0
   :1v1_attempted     0
   :1v1_won           0
   :rounds_with_kills {0 0 1 0 2 0 3 0 4 0 5 0}
   :weapons           {}})

(defn cleanup-stats [stats]
  (let [make-weapons-list (fn [stats] (assoc stats :weapons (for [[k v] (:weapons stats)] (assoc v :name k))))]
    (-> stats
        (assoc :hs_percent (* (/ (float (:hs stats)) (:kills stats)) 100))
        (dissoc :steamid)
        (make-weapons-list))))

(defn stats-for-demo [demo steamid]
  (-> (update-stats-with-demo (initial-stats steamid) demo)
      (cleanup-stats)))

(defn filter-demos [demo-type demos]
  (filter #(if (contains? #{"valve" "faceit" "esea"} demo-type) (= demo-type (:type %)) true) demos))

(defn get-stats-for-steamid [steamid demo-type]
  (->
    (->> (vals (get player-demos steamid))
         (filter-demos demo-type)
         (reduce update-stats-with-demo (initial-stats steamid)))
    (cleanup-stats)))

(defn add-score [demo]
  (let [reverse? (= 3 (team-number (:steamid demo) (first (:rounds demo))))]
    (assoc demo
      :score (if reverse? (vec (reverse (:score demo))) (:score demo))
      :outcome (name (demo-outcome demo (:steamid demo)))
      :surrendered (:surrendered demo)
      :mm_rank_update (get-in demo [:mm_rank_update (:steamid demo)]))))

(defn append-demo-stats [demo]
  (let [stats (stats-for-demo demo (:steamid demo))]
    (-> (add-score demo)
        (merge stats))))

(defn get-demos-for-steamid [steamid demo-type]
  (->> (sorted-demos-for-steamid steamid)
       (filter-demos demo-type)
       (map #(assoc % :steamid steamid))
       (map append-demo-stats)
       (map #(dissoc % :players :rounds :steamid :detailed_score :tickrate :rounds_with_kills
                     :1v1_attempted :1v1_won :weapons :tied :won :lost))
       (map #(assoc % :path (demo-path (:demoid %))))))

(defn get-demo-stats [demoid]
  (let [demo (get demos demoid)]
    (->
      (->> (map #(assoc (stats-for-demo demo (first %))
                  :team (:team (second %))
                  :steamid (str (first %))
                  :name (:name (second %))
                  :mm_rank_update (get-in demo [:mm_rank_update (first %)]))
                (seq (:players demo)))
           (group-by #(:team %))
           (assoc (select-keys demo [:score :winner :surrendered :detailed_score :timestamp :duration :map :type]) :teams))
      (merge {:rounds (map #(select-keys % [:tick]) (:rounds demo))
              :path   (demo-path demoid)}))))

; Search round

(defn get-filters [filters regexp build-filter]
  (let [regexp-demos (remove nil? (map #(re-find regexp %) filters))]
    (map #(build-filter %) regexp-demos)))

(defn not-tk [death round demo]
  (let [players (:players demo)]
    (not= (:team (get players (:victim death)))
          (:team (get players (:attacker death))))))

(defn kill-filter [regexp-demo]
  (fn [round steamid demo]
    (let [kills-no (Integer/parseInt (second regexp-demo))
          kills (filter #(and (not-tk % round demo) (= (:attacker %) steamid)) (:deaths round))
          seconds (if (nth regexp-demo 2) (Integer/parseInt (nth regexp-demo 2)) 9999999)]
      (and (<= kills-no
               (count kills))
           (reduce #(or % %2)
                   (map #(< (- (:tick (last %)) (:tick (first %)))
                            (seconds-to-ticks seconds (:tickrate demo)))
                        (partition kills-no 1 kills)))))))

; TODO demo unused here, remove?
(defn side-filter [regexp-demo]
  (fn [round steamid demo]
    (let [team (get {"ct" 3 "t" 2} (second regexp-demo))]
      (= team (team-number steamid round)))))

(defn clutch-filter [regexp-demo]
  (let [enemies-str (second regexp-demo)]
    (build-clutch-round-fn (Integer/parseInt enemies-str) false true)))

(defn weapon-filter [regexp-demo]
  (let [multiplier (if (second regexp-demo) (Integer/parseInt (second regexp-demo)) 1)
        penetrated (not (nil? (nth regexp-demo 3)))
        headshot (not (nil? (nth regexp-demo 4)))
        weapon (nth regexp-demo 2)]
    (fn [round steamid demo]
      (let [kills (filter #(and (= steamid (:attacker %))
                                (not-tk % round demo)
                                (= weapon (weapon-name (:weapon %)))
                                (if penetrated (> (:penetrated %) 0) true)
                                (if headshot (:headshot %) true))
                          (:deaths round))]
        (>= (count kills) multiplier)))))

(defn get-round-filters [filters]
  (flatten
    (map #(get-filters filters (second %) (first %))
         [[kill-filter #"^(1|2|3|4|5)k(?:<(\d+)s)?$"]
          [side-filter #"^(ct|t)$"]
          [clutch-filter #"^1v(1|2|3|4|5)$"]
          [weapon-filter (re-pattern (str "^(?:(1|2|3|4|5)(?:x)?)?("
                                          (str/join "|" weapon-names)
                                          ")(bang)?(hs)?$"))]])))

(defn round-kills [round steamid demo]
  (reduce #(let [key {:weapon (weapon-name (:weapon %2)) :headshot (:headshot %2) :penetrated (pos? (:penetrated %2))}]
            (assoc % key (+ 1 (get % key 0))))
          {} (filter #(and (= (:attacker %) steamid) (not-tk % round demo)) (:deaths round))))

(defn filter-rounds [demo steamid filters]
  (let [rounds (add-round-numbers (:rounds demo))
        filter-round (fn [round] (reduce #(and % (%2 round steamid demo)) true filters))
        filtered-rounds (filter filter-round rounds)
        demo-info (select-keys demo [:timestamp :map :demoid])
        make-kill-obj #(merge (first %) {:kills (second %)})]
    (map #(merge demo-info (hash-map :round (:number %)
                                     :steamid steamid
                                     :tick (+ (:tick %) (seconds-to-ticks 15 (:tickrate demo)))
                                     :won (= (team-number steamid %) (:winner %))
                                     :side (get {2 "T" 3 "CT"} (team-number steamid %))
                                     :kills (map make-kill-obj (round-kills % steamid demo))
                                     :path (demo-path (:demoid demo))))
         filtered-rounds)))

(defn replace-aliases [s]
  (reduce #(str/replace % (first %2) (second %2)) s {"hsbang"   "banghs"
                                                     "ace"      "5k"
                                                     "juandeag" "deaglehs"}))

(defn re-filters [re filters]
  (filter #(re-matches re %) filters))

(defn steamid-filter [filters]
  (let [steamid (first (re-filters #"\d{16,}" filters))]
    (if steamid
      (Long/parseLong steamid)
      nil)))

(defn search-demos [filters]
  (let [steamid (steamid-filter filters)
        map (first (re-filters #"(de|cs)_\w+" filters))
        demos (vals
                (if steamid
                  (get player-demos steamid)
                  demos))]
    (sort #(compare (:timestamp (first %2)) (:timestamp (first %)))
          (if map
            (filter #(= (:map %) map) demos)
            demos))))

(defn search-rounds [search-string]
  (let [search-string (replace-aliases search-string)
        filters (remove str/blank? (str/split (str/lower-case search-string) #" |\+"))
        steamid (steamid-filter filters)
        demos (search-demos filters)
        round-filters (get-round-filters filters)]
    (take 100 (mapcat #(filter-rounds (first %) (second %) round-filters)
                      (for [demo demos steamid (if steamid [steamid] (keys (:players demo)))] [demo steamid])))))

;(defn all-weapon-names []
;  (->>
;    (db-json-to-dict (jdbc/query db [(str "SELECT data FROM demos")]))
;    (mapcat #(get-in % [:data :rounds]))
;    (mapcat #(:deaths %))
;    (map #(:weapon %))
;    (set)))

(defn init-cache []
  (doseq [demo (db/get-all-demos)]
    (add-demo demo)))
