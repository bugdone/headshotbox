import { PlayerStats } from '@/types/player';

export type RankUpdate = {
  numWins: number;
  rankChange: number;
  rankNew: number;
  rankOld: number;
};

export type DemoResponse = {
  assists: number;
  assistsFlash: number;
  bannedPlayers: number;
  damage: number;
  deaths: number;
  demoid: number;
  entryKills: number;
  entryKillsAttempted: number;
  folder: string;
  hs: number;
  hsPercent: number;
  kills: number;
  map: string;
  mmRankUpdate: RankUpdate;
  openKills: number;
  openKillsAttempted: number;
  outcome: string;
  path: string;
  rating: number;
  roundsT: number;
  roundsWithDamageInfo: number;
  rws: number;
  score: number[];
  surrendered: boolean;
  timestamp: number;
  type: string;
  winner: number;
};

export type DemoApiResponse = {
  demoCount: number;
  demos: DemoResponse;
};

export type DemoStats = {
  demoid: number;
  detailedScore: number[][];
  map: string;
  path: string;
  rounds: {
    tick: number;
  }[];
  score: number[];
  surrendered: boolean;
  teams: {
    2: PlayerStats[];
    3: PlayerStats[];
  };
  timestamp: number;
  type: string;
  winner: number;
};
