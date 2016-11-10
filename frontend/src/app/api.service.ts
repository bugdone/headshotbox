import { Injectable } from '@angular/core';
import { Http, URLSearchParams, RequestOptionsArgs } from '@angular/http';
import { environment } from '../environments/environment';

import 'rxjs/add/operator/toPromise';

export class Player {
  steamid: string;
  name: string;
  demos: number;
  last_rank: number;
  last_timestamp: number;
  steam_info?: SteamInfo;
}

export class Players {
  player_count: number;
  players: Player[];
}

export class SteamInfo {
  DaysSinceLastBan: number;
  NumberOfGameBans: number;
  NumberOfVACBans: number;
  avatar: string;
  avatarfull: string;
  last_rank: number;
  personaname: string;
  timestamp: number;
}

export interface SteamInfoMap {
  [steamid: string]: SteamInfo;
}

export class DemoFilters {
  folder: string;
  demoType: string;
  mapName: string;
  rounds: string;
  startDate: number;
  endDate: number;
  teammates: string[];
}

export class Teammate {
  demos: number;
  name: string;
  steamid: string;
}

function urlSearchParams(obj: any): URLSearchParams {
  let params = new URLSearchParams();
  for (let k of Object.keys(obj)) {
    if (obj[k]) {
      params.set(k, String(obj[k]));
    }
  }
  return params;
}

@Injectable()
export class ApiService {
  private folders: string[] = null;
  private steam_info: SteamInfoMap = {};

  constructor(private http: Http) { }

  getPlayers(folder: string, offset: number, limit: number): Promise<Players> {
    let params = new URLSearchParams();
    params.set('offset', String(offset));
    params.set('limit', String(limit));
    if (folder !== null) {
      params.set('folder', folder);
    }
    // TODO cache steam_info
    return this.getPromise('/players', {search: params});
  }

  getSteamInfo(steamids: string[]): Promise<SteamInfoMap> {
    let params = new URLSearchParams();
    params.set('steamids', steamids.join(','));
    return this.getPromise('/steamids/info', {search: params}).then(data => {
      for (let steamid of Object.keys(data)) {
        this.steam_info[steamid] = data[steamid];
      }
      return data;
    });
  }

  fillSteamInfo(players: any[]): Promise<any> {
    let missing = players.filter(p => !p.steam_info && !this.steam_info[p.steamid]).map(p => p.steamid);
    let p = missing.length ? this.getSteamInfo(missing) : Promise.resolve([]);
    return p.then(_ => this.cachedSteamInfo(players));
  }

  private cachedSteamInfo(players: any[]) {
    for (let player of players) {
      if (this.steam_info[player.steamid]) {
        player.steam_info = this.steam_info[player.steamid];
      }
    }
  }

  getFolders(): Promise<string[]> {
    // Cache folders
    if (this.folders) {
      return new Promise(resolve => resolve(this.folders));
    } else {
      return this.getPromise('/folders').then(data => this.folders = data);
    }
  }

  getVersion(): Promise<{current: string, latest: string}> {
    return this.getPromise('/version');
  }

  getAuthorization(): Promise<{authorized: boolean, showLogin: boolean}> {
    return this.getPromise('/authorized');
  }

  getPlayerStats(steamid: string, demoFilters: DemoFilters) {
    // TODO Type this by renaming 1v1_* ?
    return this.getPromise('/player/' + steamid + '/stats',
                           {search: urlSearchParams(demoFilters)});
  }

  getPlayerMaps(steamid: string): Promise<string[]> {
    return this.getPromise('/player/' + steamid + '/maps');
  }

  getPlayerTeammates(steamid: string): Promise<Teammate[]> {
    return this.getPromise('/player/' + steamid + '/teammates');
  }

  private getPromise(path: string, options?: RequestOptionsArgs): Promise<any> {
    return this.http.get(environment.apiUrl + path, options).toPromise().then(r => r.json()).catch(this.handleError);
  }

  private handleError(error: any): Promise<any> {
    return Promise.reject(error.message || error);
  }
}
