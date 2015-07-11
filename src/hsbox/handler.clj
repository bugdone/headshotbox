(ns hsbox.handler
  (:require [compojure.core :refer :all]
            [compojure.handler :as handler]
            [compojure.route :as route]
            [hsbox.stats :as stats]
            [hsbox.indexer :as indexer]
            [hsbox.db :as db]
            [hsbox.version :as version]
            [hsbox.steamapi :as steamapi]
            [ring.middleware.not-modified :refer [wrap-not-modified]]
            [ring.middleware.cors :refer [wrap-cors]]
            [ring.middleware.json :refer [wrap-json-body
                                          wrap-json-response]]
            [ring.util.response :refer [response redirect]]
            [taoensso.timbre :as timbre]))

(timbre/refer-timbre)

(defn parse-filters [{:keys [startDate endDate demoType mapName teammates]}]
  {:start-date (if (nil? startDate) nil (Long/parseLong startDate))
   :end-date   (if (nil? endDate) nil (Long/parseLong endDate))
   :demo-type  demoType
   :map-name   mapName
   :teammates  (if (empty? teammates) #{}
                                      (set (map #(Long/parseLong %) (clojure.string/split teammates #","))))})

(defroutes api-routes
           (context "/player/:steamid" [steamid]
             (defroutes player-routes
                        (GET "/stats" req
                          (response (stats/get-stats-for-steamid
                                      (Long/parseLong (get-in req [:params :steamid]))
                                      (parse-filters (get req :params)))))
                        (GET "/demos" req
                          (response (stats/get-demos-for-steamid
                                      (Long/parseLong (get-in req [:params :steamid]))
                                      (parse-filters (get req :params)))))
                        (GET "/teammates" []
                          (response (stats/get-teammates-for-steamid (Long/parseLong steamid))))
                        (GET "/maps" []
                          (response (stats/get-maps-for-steamid (Long/parseLong steamid))))))

           (context "/demo/:demoid" [demoid]
             (defroutes demo-routes
                        (GET "/stats" []
                          (response (stats/get-demo-stats demoid)))
                        (GET "/notes" []
                          (response {:notes (db/get-demo-notes demoid)}))
                        (POST "/notes" {body :body}
                          (response (db/set-demo-notes demoid (:notes body))))))

           (GET "/round/search" [search-string]
             (response (stats/search-rounds search-string)))
           (GET "/steamids/info" [steamids]
             (response
               (if (empty? steamids)
                 {}
                 (let [steamids-list (clojure.string/split steamids #",")]
                   (if (clojure.string/blank? (db/get-steam-api-key))
                     (->>
                       (map #(Long/parseLong %) steamids-list)
                       (reduce #(assoc % %2 {:name (stats/get-player-latest-name %2)}) {}))
                     (steamapi/get-steamids-info steamids-list))))))
           (GET "/indexer" []
             (response {:running (indexer/is-running?)}))
           (POST "/indexer" {state :body}
             (indexer/set-indexing-state (:running state))
             (response "ok"))
           (GET "/config" []
             (response (db/get-config)))
           (POST "/config" {config :body}
             (indexer/set-config config)
             (response "ok"))
           (GET "/version" []
             (response {:current (version/get-version)
                        :latest  @version/latest-version}))
           (GET "/players" []
             (response (stats/get-players))))

(defn api-handlers [routes]
  (-> routes
      wrap-json-response
      (wrap-json-body {:keywords? true :bigdecimals? true})
      (wrap-cors :access-control-allow-origin #".+"
                 :access-control-allow-methods [:get :put :post :delete]
                 :access-control-allow-headers "Content-Type")))

(defroutes app-routes
           (GET "/" [] (redirect "index.html"))
           (context "/api" [] (api-handlers api-routes))
           (wrap-not-modified (route/resources "/"))
           (route/not-found "Not Found"))

(defn wrap-exception [f]
  (fn [request]
    (try (f request)
         (catch Throwable e
           (error e)
           (throw e)))))

(def app
  (-> app-routes
      wrap-exception
      handler/site))
