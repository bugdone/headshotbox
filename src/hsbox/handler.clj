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
            [ring.middleware.http-response :refer [wrap-http-response]]
            [ring.util.http-response :refer [bad-request!]]
            [ring.util.response :refer [response redirect not-found file-response header]]
            [cemerick.friend :as friend]
            [cemerick.friend.openid :as openid]
            [taoensso.timbre :as timbre])
  (:import (org.openid4java.consumer ConsumerManager InMemoryConsumerAssociationStore InMemoryNonceVerifier)))

(timbre/refer-timbre)

(def openid-settings (atom {}))

(defn set-openid-settings [{:keys [openid-realm admin-steamid]}]
  (when (and (not-empty openid-realm) (not-empty admin-steamid))
    (swap! openid-settings #(assoc % :realm openid-realm :steamid admin-steamid))))

(defn parse-filters [{:keys [startDate endDate demoType mapName teammates rounds folder]}]
  {:start-date (if (nil? startDate) nil (Long/parseLong startDate))
   :end-date   (if (nil? endDate) nil (Long/parseLong endDate))
   :folder     folder
   :demo-type  demoType
   :map-name   mapName
   :rounds     (if (nil? rounds) nil (clojure.string/lower-case rounds))
   :teammates  (if (empty? teammates) #{}
                                      (set (map #(Long/parseLong %) (clojure.string/split teammates #","))))})

(defn parse-long
  ([s default]
   (if (nil? s)
     default
     (parse-long s)))
  ([s]
   (try
     (Long/parseLong s)
     (catch NumberFormatException e
       (bad-request! (str "Invalid number: " (.getMessage e)))))))

(defn local-address? [address]
  (re-matches #"(127(.\d{1,3}){3})|([0:]*:1)" address))
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

(defn is-admin? [request]
  (or (empty? @openid-settings) (friend/authorized? #{::admin} (friend/identity request))))

(defroutes api-routes
           (context "/player/:steamid" [steamid]
             (let [steamid (parse-long steamid)]
               (defroutes player-routes
                          (GET "/stats" req
                            (response (stats/get-stats-for-steamid
                                        steamid
                                        (parse-filters (get req :params)))))
                          (GET "/demos" {:keys [params]}
                            (response (stats/get-demos-for-steamid
                                        steamid
                                        (parse-filters params)
                                        (parse-long (:offset params) 0)
                                        (parse-long (:limit params) 0))))
                          (GET "/teammates" []
                            (response (stats/get-teammates-for-steamid steamid)))
                          (GET "/banned" [only_opponents]
                            (response (stats/get-banned-players steamid only_opponents)))
                          (GET "/maps/statistics" req
                            (response (stats/get-maps-stats-for-steamid steamid (parse-filters (get req :params)))))
                          (GET "/rank_data" []
                            (response (stats/get-rank-data steamid)))
                          (GET "/maps" []
                            (response (stats/get-maps-for-steamid steamid))))))

           (context "/demo/:demoid" [demoid]
             (let [demoid (parse-long demoid)]
               (defroutes demo-routes
                          (GET "/stats" []
                            (response (stats/get-demo-stats demoid)))
                          (GET "/details" []
                            (response (stats/get-demo-details demoid)))
                          (GET "/notes" []
                            (response {:notes (db/get-demo-notes demoid)}))
                          (POST "/watch" {{steamid :steamid round :round tick :tick highlight :highlight} :body remote-addr :remote-addr}
                            (if (or (local-address? remote-addr)
                                    ; When running on a remote server users get "replays/..." links
                                    (:demowebmode (db/get-config)))
                              (let [info (launch/watch (local-address? remote-addr) demoid (Long/parseLong steamid) round tick highlight)]
                                (if info
                                  (response info)
                                  (not-found "")))
                              (not-found "need to be localhost or have WEBmode enabled")))
                          (GET "/download" []
                            (if (:demo_download_enabled (db/get-config))
                              (if (db/demoid-present? demoid)
                                (if (empty? (:demo_download_baseurl (db/get-config)))
                                  (response {:url (str "demo_download" "/" demoid)})
                                  (response {:url (str (:demo_download_baseurl (db/get-config)) "/"
                                                       (-> (get-in stats/demos [demoid :path])
                                                           (clojure.java.io/as-file)
                                                           (.getName)))})))
                              (not-found "demo_download is not enabled")))
                          (authorize-admin
                            (POST "/notes" {body :body}
                              (response (db/set-demo-notes demoid (:notes body))))))))
           (GET "/round/search" req
             (response (stats/search-rounds (get-in req [:params :search-string]) (parse-filters (get req :params)))))

           (context "/steamids/info" []
                    (defroutes steamids-info-routes
                               (GET "/status" []
                                 (response {:refreshing @stats/api-refreshing?}))
                               (GET "/" [steamids]
                                 (response
                                   (if (empty? steamids)
                                     {}
                                     (let [steamids-strings (clojure.string/split steamids #",")
                                           steamids-longs (map #(Long/parseLong %) steamids-strings)]
                                       (if (clojure.string/blank? (db/get-steam-api-key))
                                         (reduce #(assoc % %2 {:name (stats/get-player-latest-name %2)}) {} steamids-longs)
                                         (steamapi/get-steamids-info steamids-longs))))))
                               (authorize-admin
                                 (DELETE "/" []
                                   (info "Invalidating players steam info")
                                   (stats/invalidate-players-steam-info)))))
           (context "/indexer" []
             (authorize-admin
               (defroutes indexer-routes
                          (GET "/" []
                            (response {:running (indexer/is-running?)}))
                          (POST "/" {state :body}
                            (indexer/set-indexing-state (:running state))
                            (response "ok")))))
           (context "/config" []
             (GET "/" request
               (let [config (db/get-config)]
                 (response (if (is-admin? request)
                             config
                             (select-keys config [:playerlist_min_demo_count :demos_per_page])))))
             (authorize-admin
               (POST "/" {config :body}
                 (indexer/set-config config)
                 (response "ok"))))
           (context "/vdm" []
             (only-local
               (authorize-admin
                 (DELETE "/" []
                   (launch/delete-generated-files)
                   (response "ok")))))
           (GET "/authorized" request
             (response {:authorized
                        (if (empty? @openid-settings)
                          true
                          (is-admin? request))
                        :showLogin
                        (if (empty? @openid-settings)
                          false
                          (re-matches (java.util.regex.Pattern/compile (str "htt.*://" (:server-name request) ".*")) (:realm @openid-settings)))}))
           (GET "/version" []
             (response {:current (version/get-version)
                        :latest  @version/latest-version}))
           (GET "/folders" []
             (response (stats/get-folders)))
           (GET "/players" [folder offset limit]
             (response (stats/get-players folder (parse-long offset 0) (parse-long limit 5000)))))

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
           (GET "/demo_download/:demoid" [demoid]
             (if (:demo_download_enabled (db/get-config))
               (let [demoid (Long/parseLong demoid)]
                 (if (db/demoid-present? demoid)
                   (let [abs-path (get-in stats/demos [demoid :path])]
                     (header (file-response abs-path)
                             "content-disposition" (str "attachment; filename="
                                                        (hsbox.util/file-name abs-path))))
                   (not-found "demo not found")))
               (not-found "demo_download is not enabled")))
           (wrap-not-modified (route/resources "/"))
           (route/not-found "Not Found"))

(defn wrap-exception [f]
  (fn [request]
    (try (f request)
         (catch Throwable e
           (error e)
           (throw e)))))

(defn wrap-cache-control [f]
  (fn [request]
    (let [response (f request)]
      (update-in response [:headers] merge {"Pragma" "no-cache" "Cache-Control" "no-cache, must-revalidate"}))))

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
      wrap-http-response
      wrap-exception
      wrap-cache-control
      handler/site))
