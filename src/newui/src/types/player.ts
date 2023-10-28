import { RANKS } from 'src/constants/ranks';
import { RankUpdate } from '@/types/demo';

export type SteamInfoResponse = {
  avatar: string;
  avatarfull: string;
  personaname: string;
  numberOfVacBans: number;
  daysSinceLastBan: number;
  numberOfGameBans: number;
  steamid: number;
  timestamp: number;
};

export type PlayerResponse = {
  steamid: string;
  demos: number;
  lastTimestamp: number;
  lastRank: keyof typeof RANKS;
  name: string;
  steamInfo: SteamInfoResponse;
};

export type PlayerApiResponse = {
  playerCount: number;
  players: PlayerResponse[];
};

export type PlayerStats = {
  '1v1Attempted': number;
  '1v1Won': number;
  assists: number;
  assistsFlash: number;
  damage: number;
  deaths: number;
  entryKills: number;
  entryKillsAttempted: number;
  hs: number;
  hsPercent: number;
  kills: number;
  lost: number;
  lastRank: number;
  openKills: number;
  openKillsAttempted: number;
  rating: number;
  rounds: number;
  roundsT: number;
  roundsWithDamageInfo: number;
  roundsWithKills: {
    0: number;
    1: number;
    2: number;
    3: number;
    4: number;
    5: number;
  };
  rws: number;
  tied: number;
  weapons: {
    hs: number;
    kills: number;
    name: string;
  }[];
  won: number;
  mmRankUpdate?: RankUpdate;
  team?: number;
  steamid?: string;
};

export type PlayerInfoResponse = {
  [k: string]: SteamInfoResponse;
};

export type PlayerTeamMate = {
  demos: number;
  name: string;
  steamid: string;
};
