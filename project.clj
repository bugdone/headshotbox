(defproject
  hsbox "0.9.0"
  :description "Headshot Box"
  :url "http://headshotbox.github.io"
  :license {:name "Eclipse Public License"
            :url "http://www.eclipse.org/legal/epl-v10.html"}
  :dependencies [[org.clojure/clojure "1.6.0"]
                 [org.clojure/java.jdbc "0.3.6"]
                 [org.xerial/sqlite-jdbc "3.8.6"]
                 [com.mchange/c3p0 "0.9.2.1"]
                 [compojure "1.3.2"]
                 [ring-cors "0.1.6"]
                 [ring/ring-json "0.3.1"]
                 [ring/ring-jetty-adapter "1.3.2"]
                 [org.clojure/data.json "0.2.6"]
                 [com.taoensso/timbre "3.4.0"]
                 [org.clojure/tools.cli "0.3.2"]
                 [com.cemerick/friend "0.2.1"]
                 [org.flatland/protobuf "0.8.1"]]
  :git-dependencies [["https://github.com/nicknovitski/watt.git"]]
  :plugins [[lein-ring "0.9.3"]
            ;[lein-protobuf "0.4.1"]
            [lein-git-deps "0.0.2-SNAPSHOT"]]
  :ring {:handler hsbox.handler/app}

  :java-source-paths ["protosrc"]
  :javac-options ["-target" "1.6" "-source" "1.6"]
  :uberjar {:aot :all}
  :aot [hsbox.core]
  :main hsbox.core

  :profiles {:dev
             {:dependencies
              [[javax.servlet/servlet-api "2.5"]
               [ring-mock "0.1.5"]
               [criterium "0.4.3"]]}})
