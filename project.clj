(defproject
  hsbox "0.17.3"
  :description "Headshot Box"
  :url "http://headshotbox.github.io"
  :license {:name "Eclipse Public License"
            :url "http://www.eclipse.org/legal/epl-v10.html"}
  :dependencies [[org.clojure/clojure "1.8.0"]
                 [com.taoensso/encore "2.85.0"]
                 [clj-http "3.1.0"]
                 [cheshire "5.6.3"]
                 [org.clojure/java.jdbc "0.6.1"]
                 [org.xerial/sqlite-jdbc "3.14.2.1"]
                 [compojure "1.5.1"]
                 [ring-cors "0.1.8"]
                 [ring/ring-json "0.4.0"]
                 [ring/ring-jetty-adapter "1.5.0"]
                 [ring/ring-codec "1.0.1"]
                 [metosin/ring-http-response "0.8.0"]
                 [org.clojure/data.json "0.2.6"]
                 [com.taoensso/timbre "4.7.4"]
                 [org.clojure/tools.cli "0.3.5"]
                 [com.cemerick/friend "0.2.3"]
                 [watt "0.1.0-SNAPSHOT"]
                 [org.flatland/protobuf "0.8.1"]]
  :plugins [[lein-ring "0.9.3"]]
  :ring {:handler hsbox.handler/app}

  :java-source-paths ["protosrc", "src/hsbox/java"]
  :javac-options ["-target" "1.7" "-source" "1.7"]
  :uberjar {:aot :all}
  :aot [hsbox.core]
  :main hsbox.core

  :profiles {:dev
             {:dependencies
              [[javax.servlet/servlet-api "2.5"]
               [ring-mock "0.1.5"]
               [criterium "0.4.4"]]}})
