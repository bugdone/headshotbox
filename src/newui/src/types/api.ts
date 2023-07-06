export type ApiRequestParams = {
  limit?: number;
  offset?: number;
  orderBy?: string;
  folder?: string;
};

export type StatsRequestParams = {
  demoType?: string;
  folder?: string;
  teammates?: string[];
  mapName?: string;
  startDate?: string;
  endDate?: string;
};
