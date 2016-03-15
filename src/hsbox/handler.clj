(ns hsbox.handler
  (:require [compojure.core :refer :all]
            [compojure.handler :as handler]
            [compojure.route :as route]
            [hsbox.stats :as stats]
            [hsbox.indexer :as indexer]
            [hsbox.db :as db]
            [hsbox.launch :as launch]
            [hsbox.version :as version]
            [hsbox.steamapi :as steamapi]
            [ring.middleware.not-modified :refer [wrap-not-modified]]
            [ring.middleware.cors :refer [wrap-cors]]
            [ring.middleware.json :refer [wrap-json-body
                                          wrap-json-response]]
            [ring.util.response :refer [response redirect not-found]]
            [cemerick.friend :as friend]
            [cemerick.friend.openid :as openid]
            [taoensso.timbre :as timbre])
  (:import (org.openid4java.consumer ConsumerManager InMemoryConsumerAssociationStore InMemoryNonceVerifier)))

(timbre/refer-timbre)

(def openid-settings (atom {}))

(defn set-openid-settings [{:keys [openid-realm admin-steamid]}]
  (when (and (not-empty openid-realm) (not-empty admin-steamid))
    (swap! openid-settings #(assoc % :realm openid-realm :steamid admin-steamid))))

(defn parse-filters [{:keys [startDate endDate demoType mapName teammates rounds]}]
  {:start-date (if (nil? startDate) nil (Long/parseLong startDate))
   :end-date   (if (nil? endDate) nil (Long/parseLong endDate))
   :demo-type  demoType
   :map-name   mapName
   :rounds     (if (nil? rounds) nil (clojure.string/lower-case rounds))
   :teammates  (if (empty? teammates) #{}
                                      (set (map #(Long/parseLong %) (clojure.string/split teammates #","))))})

(defn local-address? [address]
  (re-matches #"127.0.0.\d{1,3}" address))

(defn only-local [handler]
  (fn [request]
    (if (local-address? (:remote-addr request))
      (handler request)
      (not-found ""))))

(defn authorize-admin [handler]
  (fn [request]
    (if (empty? @openid-settings)
     (handler request)
     ((friend/wrap-authorize handler #{::admin}) request))))

(defroutes api-routes
           (context "/player/:steamid" [steamid]
             (let [steamid (Long/parseLong steamid)]
               (defroutes player-routes
                          (GET "/stats" req
                            (response (stats/get-stats-for-steamid
                                        steamid
                                        (parse-filters (get req :params)))))
                          (GET "/demos" req
                            (response (stats/get-demos-for-steamid
                                        steamid
                                        (parse-filters (get req :params)))))
                          (GET "/teammates" []
                            (response (stats/get-teammates-for-steamid steamid)))
                          (GET "/banned" [only_opponents]
                            (response (stats/get-banned-players steamid only_opponents)))
                          (GET "/maps/statistics" req
                            (response (stats/get-maps-stats-for-steamid steamid (parse-filters (get req :params)))))
                          (GET "/maps" []
                            (response (stats/get-maps-for-steamid steamid))))))

           (context "/demo/:demoid" [demoid]
             (defroutes demo-routes
                        (GET "/stats" []
                          (response (stats/get-demo-stats demoid)))
                        (GET "/details" []
                          (response (stats/get-demo-details demoid)))
                        (GET "/notes" []
                          (response {:notes (db/get-demo-notes demoid)}))
                        (only-local
                          (authorize-admin
                            (POST "/watch" {{steamid :steamid round :round tick :tick highlight :highlight} :body}
                              (let [info (launch/watch demoid (Long/parseLong steamid) round tick highlight)]
                                (if info
                                  (response info)
                                  (not-found ""))))))
                        (authorize-admin
                          (POST "/notes" {body :body}
                            (response (db/set-demo-notes demoid (:notes body)))))))
           (GET "/round/search" req
             (response (stats/search-rounds (get-in req [:params :search-string]) (parse-filters (get req :params)))))
           (GET "/steamids/info" [steamids]
             (response
               (if (empty? steamids)
                 {}
                 (let [steamids-list (clojure.string/split steamids #",")
                       steamids-info (if (clojure.string/blank? (db/get-steam-api-key))
                                       (->>
                                         (map #(Long/parseLong %) steamids-list)
                                         (reduce #(assoc % %2 {:name (stats/get-player-latest-name %2)}) {}))
                                       (steamapi/get-steamids-info steamids-list))]
                   (into {} (for [[k v] steamids-info] [k (assoc v :last_rank (stats/get-last-rank (:steamid v)))]))))))
           (context "/indexer" []
             (authorize-admin
               (defroutes indexer-routes
                          (GET "/" []
                            (response {:running (indexer/is-running?)}))
                          (POST "/" {state :body}
                            (indexer/set-indexing-state (:running state))
                            (response "ok")))))
           (context "/config" []
             (authorize-admin
               (defroutes config-routes
                         (GET "/" []
                           (response (db/get-config)))
                         (POST "/" {config :body}
                           (indexer/set-config config)
                           (response "ok")))))
           (context "/vdm" []
             (only-local
              (authorize-admin
                (DELETE "/" []
                  (launch/delete-generated-files)
                  (response "ok")))))
           (GET "/authorized" request (response {:authorized
                                                 (if (empty? @openid-settings)
                                                   true
                                                   (friend/authorized? #{::admin} (friend/identity request)))
                                                 :showLogin 
                                                 (if (empty? @openid-settings)
                                                  false
                                                  (if (re-matches (java.util.regex.Pattern/compile (str "htt.*://" (:server-name request) ".*")) (:realm @openid-settings)) 
                                                    true
                                                    false))
                                                }))
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
           (GET "/openid/logout" req
             (friend/logout* (redirect (str (:context req) "/"))))
           (context "/api" [] (api-handlers api-routes))
           (wrap-not-modified (route/resources "/"))
           (route/not-found "Not Found"))

(defn wrap-exception [f]
  (fn [request]
    (try (f request)
         (catch Throwable e
           (error e)
           (throw e)))))

(defn credential-fn [stuff]
  (if (= (:identity stuff) (str "http://steamcommunity.com/openid/id/" (:steamid @openid-settings)))
    (assoc stuff :roles #{::admin})
    nil))

(defn create-secured-app []
  (let [max-nonce-age 60000
        mgr (doto (ConsumerManager.)
              ; Seems like Steam's OpenID service is using Stateless-mode.
              (.setMaxAssocAttempts 0)
              (.setAssociations (InMemoryConsumerAssociationStore.))
              (.setNonceVerifier (InMemoryNonceVerifier. (/ max-nonce-age 1000))))]
    (-> app-routes
        (friend/authenticate {:workflows
                              [(openid/workflow :openid-uri "/openid"
                                                :login-failure-handler (fn [_] (print "login-failure-handler") (redirect "index.html"))
                                                :realm (:realm @openid-settings)
                                                :credential-fn credential-fn
                                                :consumer-manager mgr)]}))))

(defn create-app []
  (-> (if (empty? @openid-settings)
        app-routes
        (create-secured-app))
      wrap-exception
      handler/site))