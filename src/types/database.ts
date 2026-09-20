export type DatabaseStatus = {
  connected: boolean;
  host: string;
  port: number;
  database: string;
  user: string;
  platform: string;
  message: string;
};
