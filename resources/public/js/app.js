var serverUrl = '/api';

var hsboxApp = angular.module('hsboxApp', ['ui.bootstrap', 'ngRoute', 'hsboxControllers', 'highcharts-ng']);

hsboxApp.config(['$routeProvider', function ($routeProvider) {
    $routeProvider.
        when('/player/:steamid', {
            templateUrl: 'templates/player.html',
            controller: 'Player'
        }).
        when('/demo/:demoid', {
            templateUrl: 'templates/demo.html',
            controller: 'Demo'
        }).
        when('/demo/:demoid/log', {
            templateUrl: 'templates/demo_log.html',
            controller: 'DemoLog'
        }).
        when('/player_list', {
            templateUrl: 'templates/player_list.html',
            controller: 'PlayerList'
        }).
        when('/round_search', {
            templateUrl: 'templates/round_search.html',
            controller: 'RoundSearch'
        }).
        when('/settings', {
            templateUrl: 'templates/settings.html',
            controller: 'Settings'
        }).
        otherwise({redirectTo: '/player_list'});
}]);

hsboxApp.config(['$compileProvider', function( $compileProvider ) {
    $compileProvider.aHrefSanitizationWhitelist(/^\s*(https?|steam):/);
}]);

hsboxApp.filter('signed', function () {
    return function (num) {
        if (num > 0)
            return "+" + num;
        else
            return "" + num;
    };
});

hsboxApp.factory('watchDemo', ['$http', function($http) {
    return function(demoid, steamid, round, tick, highlight) {
        var params = {steamid: steamid, round: round, tick: tick};
        if (highlight)
            params['highlight'] = highlight;
        $http.post(serverUrl + '/demo/' + demoid + '/watch', params).success(function(data) {
            window.location = data.url;
        });
    }
}]);

hsboxApp.factory('downloadDemo', ['$http', function($http) {
    return function(demoid) {
        window.location = serverUrl + '/demo/' + demoid + '/download';
    }
}]);
