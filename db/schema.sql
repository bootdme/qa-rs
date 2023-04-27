DROP DATABASE IF EXISTS qa;
CREATE DATABASE qa;

\c qa;

CREATE TABLE questions (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL,
    body VARCHAR,
    date_written BIGINT,
    asker_name VARCHAR,
    asker_email VARCHAR,
    reported BOOLEAN,
    helpful INTEGER
);

CREATE TABLE answers (
    id SERIAL PRIMARY KEY,
    question_id INTEGER NOT NULL REFERENCES questions(id),
    body VARCHAR,
    date_written BIGINT,
    answerer_name VARCHAR,
    answerer_email VARCHAR,
    reported BOOLEAN,
    helpful INTEGER
);

CREATE TABLE answer_photos (
    id SERIAL PRIMARY KEY,
    answer_id INTEGER NOT NULL REFERENCES answers(id),
    url VARCHAR
);

\copy questions FROM '../csv/questions.csv' WITH (FORMAT CSV, DELIMITER ",", HEADER);
\copy answers FROM '../csv/answers.csv' WITH (FORMAT CSV, DELIMITER ",", HEADER);
\copy answer_photos FROM '../csv/answers_photos.csv' WITH (FORMAT CSV, DELIMITER ",", HEADER);

CREATE INDEX questions_product_id_idx ON questions(product_id);
CREATE INDEX answers_question_id_idx ON answers(question_id);
CREATE INDEX answers_photos_id_idx ON answer_photos(answer_id);

UPDATE questions SET date_written=date_written/1000;
ALTER TABLE questions ALTER date_written TYPE TIMESTAMP WITHOUT TIME ZONE USING to_timestamp(date_written) AT TIME ZONE 'UTC';

UPDATE answers SET date_written=date_written/1000;
ALTER TABLE answers ALTER date_written TYPE TIMESTAMP WITHOUT TIME ZONE USING to_timestamp(date_written) AT TIME ZONE 'UTC';

SELECT setval('questions_id_seq', (SELECT MAX(id) FROM questions));
SELECT setval('answers_id_seq', (SELECT MAX(id) FROM answers));
SELECT setval('answer_photos_id_seq', (SELECT MAX(id) FROM answer_photos));
