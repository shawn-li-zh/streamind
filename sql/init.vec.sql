
CREATE VIRTUAL TABLE if not exists vec_concept USING vec0(
  embedding FLOAT[512] -- 这里的维度必须固定
);

CREATE VIRTUAL TABLE if not exists vec_scope USING vec0(
  embedding FLOAT[512] -- 这里的维度必须固定
);