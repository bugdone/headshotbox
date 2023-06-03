(ns hsbox.stats
  (:require [clojure.set :refer [intersection subset?]]
            [clojure.string :as str]
            [hsbox.db :as db :refer [get-config latest-data-version]]
            [hsbox.steamapi :as steamapi]
            [hsbox.util :refer [current-timestamp]])
  (:import (java.util.concurrent TimeUnit)
           (java.util.concurrent.locks ReentrantLock)))

(taoensso.timbre/refer-timbre)

(def demos {})
(def player-demos {})
(def api-refresh-lock (ReentrantLock.))
(def api-refresh-cond (.newCondition api-refresh-lock))
(def api-refreshing? (atom false))
(def refresh-all-players? (atom false))

(defn seconds-to-ticks [seconds tickrate] (int (* seconds (/ 1 tickrate))))

(defn enumerate [s]
  (map vector (range) s))

(defn get-player-name-in-demo [steamid demo]
  (get-in demo [:players steamid :name]))

(defn sorted-demos-for-steamid [steamid]
  (sort #(compare (:timestamp %) (:timestamp %2)) (vals (get player-demos steamid))))

(defn get-rank-data [steamid]
  (->>
    (sorted-demos-for-steamid steamid)
    (map #(hash-map :timestamp (:timestamp %)
                    :mm_rank_update (get-in % [:mm_rank_update steamid])))
    (filter #(not (nil? (:mm_rank_update %))))))

(defn get-last-rank [steamid]
  (->>
    (get-rank-data steamid)
    (last)
    (#(get-in % [:mm_rank_update :rank_new]))))

(defn get-players
  "Returns a list of players filtered by the parameters.

  Each player will have steamid info attached if it's present in the database"
  [folder offset limit order-by]
  (let [folder-filtered (fn [m] (if (nil? folder)
                                  m
                                  (select-keys m (for [[k v] m :when (= (:folder v) folder)] k))))
        sort-by-date (fn [c] (sort #(compare (:timestamp %2) (:timestamp %)) c))
        order-field (case order-by
                      "last_rank" :last_rank
                      "last_timestamp" :last_timestamp
                      "demos" :demos)
        players (->> player-demos
                     (reduce-kv #(let [filtered-demos (vals (folder-filtered %3))]
                                  (conj % {:steamid        %2
                                           :demos          (count filtered-demos)
                                           :last_timestamp (-> (sort-by-date filtered-demos) (first) (:timestamp))
                                           :last_rank      (get-last-rank %2)
                                           :name           (get-player-name-in-demo %2 (first filtered-demos))})) [])
                     (filter #(>= (:demos %) (:playerlist_min_demo_count (get-config) 2)))
                     (sort #(compare (get %2 order-field) (get % order-field))))
        player_count (count players)
        players (->> players
                     (drop offset)
                     (take limit))
        steam-info (steamapi/get-steamids-info-cached (map :steamid players))
        players (map #(assoc % :steam_info (get steam-info (:steamid %))) players)]
    {:player_count player_count
     :players      (map #(assoc % :steamid (str (:steamid %))) players)}))

(defn get-maps-for-steamid [steamid]
  (set (map #(:map %) (vals (get player-demos steamid)))))

(defn get-player-latest-name [steamid]
  (get-player-name-in-demo steamid (first (sorted-demos-for-steamid steamid))))

(defn get-players-in-team [demo steamid same-team?]
  (let [team (get-in demo [:players steamid :team])]
    (set (for [p (:players demo) :when (and (not= (key p) steamid) ((if same-team? = not=) (:team (val p)) team))] (key p)))))

(defn get-teammates [demo steamid]
  (get-players-in-team demo steamid true))

(defn get-teammates-for-steamid [steamid]
  (letfn [(update-teammates [teammates demo]
            (reduce #(assoc % %2 (inc (get % %2 0))) teammates (get-teammates demo steamid)))]
    (->>
      (reduce #(update-teammates % %2) {} (vals (get player-demos steamid)))
      (map #(hash-map :steamid (str (key %)) :demos (val %) :name (get-player-latest-name (key %))))
      (filter #(> (:demos %) 1))
      (sort #(compare (:demos %2) (:demos %))))))

(defn add-demo [demo]
  ; TODO use atom
  (def hsbox.stats/demos (assoc demos (:demoid demo) demo))
  (doseq [steamid (keys (:players demo))]
    (def hsbox.stats/player-demos (assoc-in player-demos [steamid (:demoid demo)] demo))))

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
    (.startsWith name "m4a1_silencer") "m4a1"
    :else name))

(def weapon-names ["mag7" "mp9" "inferno" "elite" "g3sg1" "negev" "fiveseven" "hkp2000" "m4a1" "galil" "xm1014"
                   "ssg08" "nova" "flashbang" "ak47" "m4a4" "tec9" "decoy" "hegrenade" "taser" "famas" "ump45"
                   "mp7" "worldspawn" "cz75a" "glock" "sg556" "mac10" "sawedoff" "p90" "usp" "p250" "scar20" "mp5sd"
                   "deagle" "awp" "aug" "world" "m249" "bizon" "knife" "smokegrenade" "molotov_projectile" "revolver"])

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

(defn update-stats-with-death [stats {:keys [attacker victim assister weapon headshot assistedflash]}]
  (let [steamid (:steamid stats)
        weapon (weapon-name weapon)
        enemies? (not= (get (:players stats) steamid) (get (:players stats) victim))
        killed? (and (= steamid attacker) enemies?)
        assist? (and (= steamid assister) enemies?)
        init-hs-if-needed (fn [stats weapon]
                            (if (and killed? (nil? (get-in stats [:weapons weapon :hs])))
                              (assoc-in stats [:weapons weapon :hs] 0)
                              stats))]
    (-> stats
        (inc-stat-maybe :kills killed?)
        (inc-stat-maybe :kills-this-round killed?)
        (inc-stat-maybe :deaths (= steamid victim))
        (inc-stat-maybe :assists assist?)
        (inc-stat-maybe :assists_flash (and assist? assistedflash))
        (inc-stat-maybe [:weapons weapon :kills] killed?)
        (init-hs-if-needed weapon)
        (inc-stat-maybe :hs (and headshot killed?))
        (inc-stat-maybe [:weapons weapon :hs] (and headshot killed?)))))

(defn team-number [steamid round]
  (get (:players round) steamid 0))

(defn deaths-until-round-end [round]
  (if (:tick round)
    (filter #(<= (:tick %) (:tick_end round)) (:deaths round))
    (:deaths round)))

(defn build-clutch-round-fn [enemies exact-enemies? won?]
  (fn [round steamid & [demo]]
    (let [deaths (deaths-until-round-end round)
          player-team (team-number steamid round)
          player-death (split-with #(not= (:victim %) steamid) deaths)
          same-team (fn [death] (= player-team (team-number (:victim death) round)))
          not-same-team (comp not same-team)
          last-teammate-death (split-with not-same-team
                                          (reverse (first player-death)))
          dead-teammates (count (filter same-team (second last-teammate-death)))
          alive-enemies (- 5 (count (filter not-same-team (second last-teammate-death))))]
      (and ((if exact-enemies? = >=) alive-enemies enemies) (= 4 dead-teammates) (if won? (= (:winner round) player-team) true)))))

(defn get-rws [steamid round]
  (let [team (team-number steamid round)
        is-t (= 2 team)]
    (if (and (not-empty (:damage round)) (= team (:winner round)))
      ; reason 1 for #SFUI_Notice_Target_Bombed
      (let [bomb (or (and (:bomb_exploded round) (= 1 (:win_reason round))) (:bomb_defused round))
            dmg-ratio (if bomb 70 100)
            team-damage (reduce-kv #(+ % (if (= team (get-in round [:players %2])) %3 0))
                                   0
                                   (:damage round))]
        (+ (if (and bomb (or (= steamid (:bomb_defused round))
                             (= steamid (:bomb_exploded round))))
             30
             0)
           (if (not= team-damage 0)
             (* dmg-ratio (/ (float (get-in round [:damage steamid] 0)) team-damage))
             ; if team won without doing any damage, assign each player an equal damage share
             (* dmg-ratio 0.2))))
      0)))

(defn update-stats-with-round [stats round]
  (if ((get stats :round-filter #(or true % %2)) round (:steamid stats))
    (let [steamid (:steamid stats)
          updated-stats (reduce update-stats-with-death
                                (assoc stats :kills-this-round 0 :players (:players round))
                                (:deaths round))
          multikills (:kills-this-round updated-stats)
          first-death (first (:deaths round))
          first-dead (= steamid (:victim first-death))
          first-killer (= steamid (:attacker first-death))
          team (team-number steamid round)
          is-t (= 2 team)
          rws (get-rws steamid round)]
      (if (> multikills 5)
        (do
          (error "Error in demo" (:demoid stats) ":" steamid "had" multikills "kills in round" (:number round))
          stats)
        (-> updated-stats
            (dissoc :kills-this-round :players)
            (update-in [:rounds_with_kills multikills] inc)
            (update-in [:damage] #(+ % (get-in round [:damage steamid] 0)))
            (update-in [:rws] #(+ % rws))
            (inc-stat-maybe :rounds true)
            (inc-stat-maybe :rounds_with_damage_info (not-empty (:damage round)))
            (inc-stat-maybe :rounds_t is-t)
            (inc-stat-maybe :entry_kills_attempted (and is-t (or first-dead first-killer)))
            (inc-stat-maybe :entry_kills (and is-t first-killer))
            (inc-stat-maybe :open_kills_attempted (or first-dead first-killer))
            (inc-stat-maybe :open_kills first-killer)
            (inc-stat-maybe :1v1_attempted ((build-clutch-round-fn 1 true false) round steamid))
            (inc-stat-maybe :1v1_won ((build-clutch-round-fn 1 true true) round steamid)))))
    stats))

(defn demo-outcome [demo steamid]
  (cond
    (= (:winner demo) 0) :tied
    (= (:winner demo) (get-in demo [:players steamid :team])) :won
    true :lost))

(defn add-stat [stats stat value]
  (assoc stats stat (+ (stats stat) value)))

(defn add-round-numbers [rounds]
  (map #(assoc (second %) :number (+ 1 (first %))) (enumerate rounds)))

(defn update-stats-with-demo [stats demo]
  (-> (reduce update-stats-with-round (assoc stats :demoid (:demoid demo)) (add-round-numbers (:rounds demo)))
      (add-stat (demo-outcome demo (:steamid stats)) 1)
      (dissoc :demoid)
      (add-hltv-rating)))

(defn initial-stats [steamid]
  {:steamid                 steamid
   :kills                   0
   :deaths                  0
   :assists                 0
   :assists_flash           0
   :rounds                  0
   :rounds_t                0
   :won                     0
   :lost                    0
   :tied                    0
   :hs                      0
   :1v1_attempted           0
   :1v1_won                 0
   :entry_kills             0
   :entry_kills_attempted   0
   :open_kills              0
   :open_kills_attempted    0
   :rounds_with_kills       {0 0 1 0 2 0 3 0 4 0 5 0}
   :damage                  0
   :rounds_with_damage_info 0
   :rws                     0
   :weapons                 {}})

(defn cleanup-stats [stats]
  (let [make-weapons-list (fn [stats] (assoc stats :weapons (for [[k v] (:weapons stats)] (assoc v :name k))))]
    (-> stats
        (assoc :hs_percent (* (/ (float (:hs stats)) (:kills stats)) 100))
        (dissoc :steamid :round-filter)
        (make-weapons-list))))

(defn stats-for-demo [demo steamid]
  (-> (update-stats-with-demo (initial-stats steamid) demo)
      (cleanup-stats)))

(defn filter-demos [steamid {:keys [folder demo-type start-date end-date map-name teammates]} demos]
  (filter #(and
            (if folder (= (:folder %) folder) true)
            (if (contains? (-> latest-data-version keys set) demo-type) (= demo-type (:type %)) true)
            (if map-name (= (:map %) map-name) true)
            (if start-date (>= (:timestamp %) start-date) true)
            (if end-date (<= (:timestamp %) end-date) true)
            (if (empty? teammates) true (subset? teammates (get-teammates % steamid))))

          demos))

(defn update-map-stats-with-demo [stats demo]
  (let [steamid (:steamid stats)
        rounds-stats (reduce
                       #(let [round-team (team-number steamid %2)]
                         (-> %
                             (inc-stat-maybe :t_rounds (= 2 round-team))
                             (inc-stat-maybe :ct_rounds (= 3 round-team))
                             (inc-stat-maybe :ct_rounds_won (and (= 3 round-team (:winner %2))))
                             (inc-stat-maybe :t_rounds_won (and (= 2 round-team (:winner %2))))))
                       {}
                       (:rounds demo))
        stats (assoc stats (:map demo) (merge-with
                                         +
                                         rounds-stats
                                         (get stats (:map demo) {:t_rounds         0
                                                                 :ct_rounds        0
                                                                 :t_rounds_won     0
                                                                 :ct_rounds_won    0
                                                                 :played           0
                                                                 :won              0
                                                                 :lost             0
                                                                 :won_starting_ct  0
                                                                 :lost_starting_ct 0})))
        inc-map-stat-maybe (fn [stats stat pred] (inc-stat-maybe stats [(:map demo) stat] pred))
        outcome (demo-outcome demo steamid)
        team (team-number steamid (first (:rounds demo)))]
    (-> stats
        (inc-map-stat-maybe :played true)
        (inc-map-stat-maybe :won (= :won outcome))
        (inc-map-stat-maybe :lost (= :lost outcome))
        (inc-map-stat-maybe :won_starting_ct (and (= :won outcome) (= 3 team)))
        (inc-map-stat-maybe :lost_starting_ct (and (= :lost outcome) (= 3 team))))))

(defn get-maps-stats-for-steamid [steamid filters]
  (->
    (->> (vals (get player-demos steamid))
         (filter-demos steamid filters)
         (reduce update-map-stats-with-demo {:steamid steamid}))
    (dissoc :steamid)))

(defn get-round-filter [filters]
  (let [filter (:rounds filters)]
    (cond
      (= filter "pistol") (fn [round _] (#{1 16} (:number round)))
      (= filter "t") #(= 2 (get-in % [:players %2]))
      (= filter "ct") #(= 3 (get-in % [:players %2]))
      :else (fn [_ _] true))))

(defn get-stats-for-steamid [steamid filters]
  (->
    (->> (vals (get player-demos steamid))
         (filter-demos steamid filters)
         (reduce update-stats-with-demo
                 (->
                   (initial-stats steamid)
                   (assoc :round-filter (get-round-filter filters)))))
    (cleanup-stats)
    (assoc :last_rank (get-last-rank steamid))))

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

(defn get-banned-players [steamid only-opponents? filters]
  (let [demos (->>
                (vals (get player-demos steamid))
                (filter-demos steamid filters))
        get-team (fn [demo steamid] (get-in demo [:players steamid :team]))
        get-players-data (fn [demo]
                           (map
                             #(vector % (:timestamp demo) (not= (get-team demo steamid) (get-team demo %)))
                             (->
                               (set (concat (get-players-in-team demo steamid false)
                                            (if only-opponents?
                                              []
                                              (get-players-in-team demo steamid true))))
                               (disj steamid))))
        played-with (mapcat get-players-data demos)
        players (reduce #(let [already (get % (first %2) {:timestamp 0})]
                          (assoc % (first %2) {:timestamp (max (second %2) (:timestamp already))
                                               :opponent  (if (< (:timestamp already) (second %2))
                                                            (last %2)
                                                            (:opponent already))}))
                        {}
                        played-with)
        steam-info (apply hash-map (mapcat #(vector (:steamid %) (dissoc % :timestamp)) (db/get-steamid-info (keys players))))
        now (current-timestamp)]
    (->>
      (filter #(let [info (get steam-info (key %))]
                (and info
                     (:NumberOfVACBans info)
                     (:NumberOfGameBans info)
                     (or (pos? (:NumberOfVACBans info)) (pos? (:NumberOfGameBans info)))
                     (>= (- now (* 3600 24 (:DaysSinceLastBan info))) (:timestamp (val %)))))
              players)
      (map #(assoc
             (merge (val %) (get steam-info (key %)))
             :steamid
             (str (key %)))))))

(defn get-banned-statistics [steamid filters]
  (let [demos (->>
                (vals (get player-demos steamid))
                (filter-demos steamid filters))
        banned-info (get-banned-players steamid false filters)
        banned-players (set (map #(Long/parseLong (:steamid %)) banned-info))
        banned-opponents (set (map #(Long/parseLong (:steamid %)) (filter #(:opponent %) banned-info)))]
    (->> demos
         (reduce #(let [date (new java.util.Date (* 1000 (:timestamp %2)))
                        key (/ (.getTimeInMillis (doto (java.util.Calendar/getInstance)
                                                   (.clear)
                                                   (.set java.util.Calendar/MONTH, (.getMonth date))
                                                   (.set java.util.Calendar/YEAR (+ 1900 (.getYear date))))) 1000)
                        in-set (fn [s] (if (empty? (intersection s (set (keys (:players %2)))))
                                         0
                                         1))]
                    (assoc % key (merge-with + (get % key {:games 0 :games_banned 0 :won 0 :lost 0})
                                             {:games                  1
                                              :games_banned           (in-set banned-players)
                                              :games_banned_opponents (in-set banned-opponents)}))) {})
         (map #(assoc (val %) :timestamp (key %)))
         (sort #(- (:timestamp %) (:timestamp %2))))))

(defn append-ban-info [steamid]
  (let [banned (get-banned-players steamid false {})]
    (fn [demo]
      (assoc
        demo
        :banned_players
        (count (intersection
                 (set (keys (:players demo)))
                 (set (map #(Long/parseLong (:steamid %)) banned))))))))

(defn get-demos-for-steamid [steamid filters offset & [limit]]
  (let [limit (or limit (:demos_per_page (get-config) 50))
        all-demos (->> (reverse (sorted-demos-for-steamid steamid))
                       (filter-demos steamid filters))
        filtered-demos (->> all-demos
                            (drop offset)
                            (#(if (or (nil? limit) (zero? limit)) % (take limit %)))
                            (map #(assoc % :steamid steamid))
                            (map append-demo-stats)
                            (map (append-ban-info steamid))
                            (map #(dissoc % :players :rounds :steamid :detailed_score :tickrate :rounds_with_kills
                                          :1v1_attempted :1v1_won :weapons :tied :won :lost :player_slots)))]
    {:demo_count (count all-demos)
     :demos      filtered-demos}))

(defn kw-long-to-str [dict path]
  (assoc-in dict path (into {} (for [[k v] (get-in dict path)] [(str k) v]))))

(defn get-demo-details [demoid]
  (let [demo (get demos demoid)
        convert-death-steamid (fn [death]
                                (->
                                  (reduce-kv #(if
                                               (get #{:assister :victim :attacker} %2)
                                               (assoc % %2 (str %3))
                                               (assoc % %2 %3))
                                             {} death)
                                  (assoc :weapon_name (weapon-name (:weapon death)))))
        convert-if-exists (fn [round key]
                            (if (get round key)
                              (assoc round key (str (get round key)))
                              round))
        compute-rws (fn [round]
                      (apply hash-map (reduce #(concat % [(str %2) (get-rws %2 round)]) [] (keys (:players round)))))]
    (->
      demo
      (kw-long-to-str [:players])
      (assoc :rounds (map #(-> %
                               (kw-long-to-str [:players])
                               (kw-long-to-str [:damage])
                               (assoc :rws (compute-rws %))
                               (convert-if-exists :bomb_exploded)
                               (convert-if-exists :bomb_defused)
                               (assoc :deaths (map convert-death-steamid (:deaths %))))
                          (:rounds demo))
             :path (:path demo)))))

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
           (assoc (select-keys demo [:score :winner :surrendered :detailed_score :timestamp :duration :map :type :demoid]) :teams))
      (merge {:rounds (map #(select-keys % [:tick]) (:rounds demo))
              :path   (:path demo)}))))

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

(defn ninja-filter [regexp-demo]
  (fn [round steamid demo]
    (let [deaths (concat (deaths-until-round-end round) (map #(hash-map :victim (first %)) (:disconnected round)))
          dead-ts (count (filter #(= 2 (team-number (:victim %) round)) deaths))
          dead-cts (- (count deaths) dead-ts)]
      (and (= (:bomb_defused round) steamid)
           (>= dead-cts dead-ts)))))

(defn not-nade? [weapon]
  (not (#{"hegrenade" "inferno" "flashbang" "smokegrenade" "decoy"} (weapon-name weapon))))

(defn scoped-weapon [weapon]
  (#{"awp" "ssg08" "scar20" "g3sg1"} (weapon-name weapon)))

(defn through-smoke? [kill]
  (and (:smoke kill) (not-nade? (:weapon kill))))

(defn no-scope? [kill]
  (and (scoped-weapon (:weapon kill)) (nil? (:scoped_since kill))))

(defn air-kill? [kill]
  (and (not-nade? (:weapon kill)) (:air_velocity kill) (>= (Math/abs (:air_velocity kill)) 1)))

(defn quick-scope? [kill demo]
  (and (scoped-weapon (:weapon kill))
       (:scoped_since kill)
       (< (* (:tickrate demo) (- (:tick kill) (:scoped_since kill))) 0.1)))

(defn weapon-filter [regexp-demo]
  (let [multiplier (if (second regexp-demo) (Integer/parseInt (second regexp-demo)) 1)
        flags? (not (nil? (nth regexp-demo 3)))
        flags (nth regexp-demo 3)
        weapon (nth regexp-demo 2)
        flag (fn [f] (and flags? (.contains flags f)))]
    (fn [round steamid demo]
      (let [same-weapon-and-tick (fn [kill]
                                   (filter #(and (= (:tick %) (:tick kill)) (= (:weapon %) (:weapon kill)) (= (:attacker %) (:attacker kill)))
                                           (:deaths round)))
            kills (filter #(and (= steamid (:attacker %))
                                (not-tk % round demo)
                                (or (= weapon (weapon-name (:weapon %))) (= weapon ""))
                                (if (flag "bang") (> (:penetrated %) 0) true)
                                (if (flag "hs") (:headshot %) true)
                                (if (flag "smoke") (through-smoke? %) true)
                                (if (flag "collateral") (> (count (same-weapon-and-tick %)) 1) true)
                                (if (flag "jump")
                                  (and (:jump %)
                                       (<= 0.1 (* (:tickrate demo) (:jump %)) 0.5)
                                       (not-nade? (:weapon %)))
                                  true)
                                (if (flag "air") (air-kill? %) true)
                                (if (flag "noscope") (no-scope? %) true)
                                (if (flag "quickscope") (quick-scope? % demo) true))
                          (:deaths round))]
        (>= (count kills) multiplier)))))

(defn get-search-round-filters [filters]
  (flatten
    (map #(get-filters filters (second %) (first %))
         [[kill-filter #"^(1|2|3|4|5)k(?:<(\d+)s)?$"]
          [side-filter #"^(ct|t)$"]
          [clutch-filter #"^1v(1|2|3|4|5)$"]
          [ninja-filter #"ninja"]
          [weapon-filter (re-pattern (str "^(?:(1|2|3|4|5)(?:x)?)?("
                                          (str (str/join "|" weapon-names) "|")
                                          ")((?:bang|hs|jump|smoke|collateral|air|noscope|quickscope)*)$"))]])))

(defn round-kills [round steamid demo]
  (reduce #(let [key {:weapon     (weapon-name (:weapon %2))
                      :headshot   (:headshot %2)
                      :penetrated (pos? (:penetrated %2))
                      :smoke      (through-smoke? %2)
                      :air        (air-kill? %2)
                      :quickscope (quick-scope? %2 demo)
                      :noscope    (no-scope? %2)}]
            (assoc % key (+ 1 (get % key 0))))
          {} (filter #(and (= (:attacker %) steamid) (not-tk % round demo)) (:deaths round))))

(defn filter-rounds [demo steamid filters]
  (let [rounds (add-round-numbers (:rounds demo))
        filter-round (fn [round] (reduce #(and % (%2 round steamid demo)) true filters))
        filtered-rounds (filter filter-round rounds)
        demo-info (select-keys demo [:timestamp :map :demoid])
        make-kill-obj #(merge (first %) {:kills (second %)})]
    (map #(merge demo-info (hash-map :round (:number %)
                                     :steamid (str steamid)
                                     :won (= (team-number steamid %) (:winner %))
                                     :side (get {2 "T" 3 "CT"} (team-number steamid %))
                                     :kills (map make-kill-obj (round-kills % steamid demo))
                                     :path (:path demo)))
         filtered-rounds)))

(defn replace-aliases [s]
  (reduce #(str/replace % (first %2) (second %2)) s {"kqly"     "jump"
                                                     "ace"      "5k"
                                                     "juandeag" "deaglehs"
                                                     (re-pattern "(?<!j)ump")      "ump45"
                                                     "cz"       "cz75a"
                                                     "mp5"      "mp5sd"
                                                     "scout"    "ssg08"}))

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
    (if map
      (filter #(= (:map %) map) demos)
      demos)))

(defn search-rounds [search-string demo-filters]
  (let [search-string (replace-aliases search-string)
        filters (remove str/blank? (str/split (str/lower-case search-string) #" |\+"))
        steamid (steamid-filter filters)
        demos (->>
                (search-demos filters)
                (filter-demos steamid demo-filters)
                (sort #(compare (:timestamp %2) (:timestamp %))))
        round-filters (conj (get-search-round-filters filters)
                            (fn [r s _] ((get-round-filter demo-filters) r s)))]
    (take 100 (mapcat #(filter-rounds (first %) (second %) round-filters)
                      (for [demo demos steamid (if steamid [steamid] (keys (:players demo)))] [demo steamid])))))

;(defn all-weapon-names []
;  (->>
;    (db-json-to-dict (jdbc/query db [(str "SELECT data FROM demos")]))
;    (mapcat #(get-in % [:data :rounds]))
;    (mapcat #(:deaths %))
;    (map #(:weapon %))
;    (set)))

(defn get-folders []
  (sort (set (map #(-> (second %) (:folder)) demos))))

(defn init-cache []
  (def hsbox.stats/demos {})
  (def hsbox.stats/player-demos {})
  (doseq [demo (db/get-all-demos)]
    (add-demo demo)))

(defn load-cache []
  (hsbox.db/init-db-if-absent)
  (init-cache))

(defn refresh-players-steam-info []
  (.lock api-refresh-lock)
  (try
    (reset! refresh-all-players? true)
    (.signal api-refresh-cond)
    (finally
      (.unlock api-refresh-lock))))

(defn update-players-steam-info []
  (while true
    (.lock api-refresh-lock)
    (try
      (try
        (reset! api-refreshing? true)
        (steamapi/get-steamids-info (keys player-demos) :refresh-all? @refresh-all-players? :delete-old? true)
        (finally
          (reset! refresh-all-players? false)
          (reset! api-refreshing? false)))
      (.await api-refresh-cond 1 TimeUnit/HOURS)
      (finally
        (.unlock api-refresh-lock)))))

(defn delete-old-demos
  "Delete demos that are not below the demo directory"
  []
  (db/keep-only (->> (db/get-demo-directory)
                     (clojure.java.io/as-file)
                     file-seq
                     (map #(.getCanonicalPath %))
                     (filter #(.endsWith % ".dem")))))
