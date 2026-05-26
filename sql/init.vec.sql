CREATE TABLE "concept" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT,
  "name" TEXT NOT NULL UNIQUE,
  "content" TEXT NOT NULL,
  "embedding" BLOB,         -- 对应 column_type = "Blob"
  "embeded" BOOLEAN NOT NULL DEFAULT 0,
  "create_time" DATETIME NOT NULL
);
CREATE VIRTUAL TABLE vec_concept USING vec0(
  id INTEGER PRIMARY KEY,
  embedding FLOAT[1536] -- 这里的维度必须固定
);