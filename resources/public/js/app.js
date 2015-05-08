var serverUrl = '/api';

var hsboxApp = angular.module('hsboxApp', ['ui.bootstrap', 'ngRoute', 'hsboxControllers']);

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

hsboxApp.filter('mydate', function () {
    return function (date) {
        if (!date)
            return '';
        d = new Date(date);
        format = {day: 'numeric', month: 'short', hour: "2-digit", minute: "2-digit", hour12: false};
        if (d.getFullYear() != (new Date()).getFullYear())
            format.year = 'numeric';
        return d.toLocaleString(undefined, format);
    };
});

function watchDemoUrl(path, steamid, tick, highlight) {
    return 'steam://rungame/730/' + steamid + '/+playdemo "' +
        path + (tick ? '@' + tick : '') + '" ' +
        (highlight ? steamid : '');
}
