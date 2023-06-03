import type { ApiRequestParams } from '@/types/api';
import type { PlayerApiResponse, PlayerInfoResponse, PlayerStats } from '@/types/player';

import { api } from 'boot/axios';

export const PlayerApi = {
  get: async (params: ApiRequestParams = { limit: 50, offset: 0 }): Promise<PlayerApiResponse> => {
    const { data } = await api.get<PlayerApiResponse>('players', {
      params,
    });

    return data;
  },

  getPlayerInfo: async (steamId: string): Promise<PlayerInfoResponse> => {
    const { data } = await api.get<PlayerInfoResponse>('steamids/info', { params: { steamids: steamId } });

    return data;
  },

  stats: async (steamId: string): Promise<PlayerStats> => {
    const { data } = await api.get<PlayerStats>(`player/${steamId}/stats`);

    return data;
  },
};
