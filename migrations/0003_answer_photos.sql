CREATE TABLE answer_photos (
    id SERIAL PRIMARY KEY,
    answer_id INTEGER NOT NULL REFERENCES answers(id),
    url VARCHAR
);

CREATE INDEX answers_photos_id_idx ON answer_photos(answer_id);
