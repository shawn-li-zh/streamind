
CREATE VIRTUAL TABLE if not exists vec_concept USING vec0(
  embedding FLOAT[1024] -- 这里的维度必须固定
);

CREATE VIRTUAL TABLE if not exists vec_scope USING vec0(
  embedding FLOAT[1024] -- 这里的维度必须固定
);


insert into vec_concept(rowid,embedding) select id, embedding from rule_concept where embedded = 1 and embedding is not null and id not in (select rowid from vec_concept);
insert into vec_scope(rowid,embedding) select id, embedding from rule_scope where embedded = 1 and embedding is not null and id not in (select rowid from vec_scope); 