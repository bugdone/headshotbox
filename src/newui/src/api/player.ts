import { api } from 'boot/axios';
import type { ApiRequestParams } from '@/types/api';
import type { PlayerApiResponse } from '@/types/player';

export const PlayerApi = {
  get: async (params: ApiRequestParams = { limit: 50, offset: 0 }): Promise<PlayerApiResponse> => {
    const { data } = await api.get<PlayerApiResponse>('players', {
      params,
    });

    return data;
  },
};
