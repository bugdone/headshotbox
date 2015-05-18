CREATE TABLE IF NOT EXISTS demos (
  demoid VARCHAR(256) PRIMARY KEY,
  timestamp INT,
  map VARCHAR(20) NOT NULL,
  data_version INT,
  data TEXT NOT NULL,
  notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_demos_map ON demos(map);
CREATE INDEX IF NOT EXISTS idx_demos_timestamp ON demos(timestamp);

CREATE TABLE IF NOT EXISTS playerdemos (
  demoid VARCHAR(256) NOT NULL REFERENCES demos(demoid),
  steamid INT NOT NULL,
  PRIMARY KEY(demoid, steamid)
);

CREATE TABLE IF NOT EXISTS steamids (
  steamid INT PRIMARY KEY,
  timestamp INT,
  data TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS meta (
  key VARCHAR(255) PRIMARY KEY,
  value TEXT
);

INSERT INTO meta (key, value) VALUES
  ('schema_version', '1'),
  ('config', '{"demo_directory": "C:\\Program Files (x86)\\Steam\\SteamApps\\common\\Counter-Strike Global Offensive\\csgo\\replays"}');
