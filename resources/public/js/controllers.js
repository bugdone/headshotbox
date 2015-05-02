var hsboxControllers = angular.module('hsboxControllers', []);

function getPlayerSummaries(steamids) {
    return serverUrl + '/steamids/info?steamids=' + steamids.join(',');
}

function demoOutcome(demoStats) {
    if (demoStats.score.winner == '2')
        outcome = 'Team A wins';
    else if (demoStats.score.winner == '3')
        outcome = 'Team B wins';
    else
        outcome = 'Draw';
    return outcome + '! ';
}

hsboxControllers.controller('Player', function ($scope, $http, $routeParams, $sce) {
    $scope.watchDemoUrl = watchDemoUrl;
    steamid = $routeParams.steamid;
    $scope.orderWeapons = '-kills';
    $scope.steamid = steamid;
    $scope.orderDemos = '-timestamp';
    $http.get(serverUrl + '/player/' + steamid + '/stats').success(function(data) {
        $scope.stats = data;
        $scope.stats.weapons.forEach(function (p) {
            p.hs_percent = (p.hs / p.kills) * 100;
        });
    });
    $http.get(serverUrl + '/player/' + steamid + '/demos').success(function(data) {
        $scope.demos = data;
        $scope.demos.forEach(function (m) {
            m.kdd = m.kills - m.deaths;
            if (!m.timestamp)
                m.timestamp = 0;
        });
    });
    $http.get(getPlayerSummaries([steamid])).success(function (response) {
        $scope.player = response[steamid];
    });
    $scope.demoStats = {}
    $scope.steamAccounts = {}
    $scope.visibleDemo = ''
    $scope.visibleRound = 0
    $scope.orderTeams = '-kills';
    $scope.resetNotesControls = function() {
        $scope.notesControls = {'demoNotesInput': '', 'demoNotesView': ''};
    };
    $scope.linkToTick = function(demo, p1) {
        tick = parseInt(p1, 10);
        round = false;
        if (demo.lastIndexOf('round', 0) === 0) {
            tick = $scope.theDemo.rounds[tick - 1].tick;
            round = true;
        }
        return "<a href='" + watchDemoUrl($scope.theDemo.path, steamid, tick) + "'>" + demo + "</a>";
    }
    $scope.addLinks = function(text) {
        text = text.replace(/(?:\r\n|\r|\n)/g, '<br />');
        return text.replace(/(?:(?:round|tick) ?)(\d+)/g, $scope.linkToTick);
    };
    $scope.updateDemoNotesView = function() {
        if (typeof $scope.notesControls.demoNotesInput != undefined)
            $scope.notesControls.demoNotesView = $sce.trustAsHtml($scope.addLinks($scope.notesControls.demoNotesInput));
    };
    $scope.updateDemoNotes2 = function() {
        $http.post(serverUrl + '/demo/' + $scope.visibleDemo + '/notes', {'notes': $scope.notesControls.demoNotesInput}).success(function() {
            $scope.updateDemoNotesView();
        });
    }

    $scope.resetNotesControls();
    $scope.doMakeVisible = function(demoid, round) {
        $scope.resetNotesControls();
        $scope.visibleDemo = demoid;
        $scope.theDemo = $scope.demoStats[demoid];
        $scope.visibleRound = round;
        $http.get(serverUrl + '/demo/' + demoid + '/notes').success(function (response) {
            if ($scope.visibleDemo == demoid) {
                $scope.demoStats[$scope.visibleDemo].notes = response.notes;
                $scope.notesControls['demoNotesInput'] = response.notes;
                $scope.updateDemoNotesView();
            }
        });
    };
    $scope.makeVisible = function(demoid, round) {
        round = typeof round !== 'undefined' ? round : 0;
        if ($scope.visibleDemo != demoid) {
            if (!$scope.demoStats[demoid]) {
                $http.get(serverUrl + '/demo/' + demoid + '/stats').success(function(data) {
                    $scope.demoStats[demoid] = data;
                    $scope.doMakeVisible(demoid, round);

                    // Compute kdd and fetch steamids data from steam
                    missingPlayers = [];
                    for (var key in $scope.theDemo.teams) {
                        if ($scope.theDemo.teams.hasOwnProperty(key)) {
                            $scope.theDemo.teams[key].forEach(function (p) {
                                p.kdd = p.kills - p.deaths;
                                if (!$scope.steamAccounts[p.steamid])
                                    missingPlayers[missingPlayers.length] = p.steamid;
                            });
                        }
                    }
                    if (missingPlayers.length) {
                        $http.get(getPlayerSummaries(missingPlayers)).success(function (response) {
                            for (var player in response) {
                                $scope.steamAccounts[player] = response[player];
                            }
                        });
                    }
                });
            } else
                $scope.doMakeVisible(demoid, round);

        }
        else if ($scope.visibleRound == round) {
            $scope.visibleRound = 0;
            $scope.visibleDemo = '';
            $scope.theDemo = '';
        } else {
            $scope.doMakeVisible(demoid, round);
        }
    };
    $scope.isVisible = function(demoid, round) {
        round = typeof round !== 'undefined' ? round : 0;
        return $scope.visibleDemo == demoid && $scope.visibleRound == round;
    };

    $scope.demoOutcome = demoOutcome;
 });

hsboxControllers.controller('PlayerList', function ($scope, $http) {
    $http.get(serverUrl + '/players').success(function (data) {
        $scope.players = data;
        var steamIds = $scope.players.map(function(p) { return p.steamid; });
        var url = getPlayerSummaries(steamIds);
        $http.get(url).success(function (response) {
            for (var i in $scope.players) {
                player = $scope.players[i];
                if (response[player.steamid]) {
                    player.avatar = response[player.steamid].avatar;
                    player.personaname = response[player.steamid].personaname;
                }
            }
        });
    });
});

hsboxControllers.controller('RoundSearch', function ($scope, $http, $routeParams) {
    $scope.setOrder = function(field) {
        if ($scope.orderRounds == field)
            $scope.orderRounds = '-' + field;
        else
           $scope.orderRounds = field;
    }
    $scope.orderRounds = '-date';
    $scope.watchDemoUrl = watchDemoUrl;
    $scope.roundHelpIsCollapsed = true;
    steamid = $routeParams.steamid;
    $scope.search_string = "";
    $scope.search = function() {
        $http.get(serverUrl + '/round/search', { params: {'search-string': steamid + ' ' + $scope.search_string} }).success(function(data) {
            $scope.rounds = data;
            $scope.rounds.forEach(function (r) {
                if (r.won)
                    r.won_str = "Yes";
                else
                    r.won_str = "No";
            });
        });
    }
});

hsboxControllers.controller('Settings', function ($scope, $http) {
    $scope.steamApiCollapsed = true;
    $scope.demoDirectoryCollapsed = true;
    $scope.getSettings = function() {
        $http.get(serverUrl + '/config').success(function(data) {
            $scope.config = data;
        });
    };
    $scope.config = $scope.getSettings();
    $scope.updateSettings = function() {
        $http.post(serverUrl + '/config', $scope.config).success(function(data) {
        });
    };

    $scope.invertIndexerState = function() {
        if (typeof $scope.indexerRunning === 'undefined')
            return;
        $http.post(serverUrl + '/indexer', {'running': !$scope.indexerRunning}).success(function(data) {
            $scope.getIndexerState();
        });
    };

    $scope.getIndexerState = function() {
        $http.get(serverUrl + '/indexer').success(function(data) {
            $scope.indexerRunning = data.running;
        });
    };

    $scope.getIndexerState();
});

hsboxControllers.controller('Navbar', function ($scope, $http, $interval) {
    $scope.active = 'player_list';
    $scope.newVersionAvailable = false;
    $scope.checkVersion = function($scope) {
        $http.get(serverUrl + '/version').success(function(data) {
            if (data.current != data.latest)
                $scope.newVersionAvailable = true;
        });
    };
    $scope.checkVersion($scope);
    $interval(function(){ $scope.checkVersion($scope); }, 1000 * 3600 * 24);
    // TODO user route params to set active?
});
