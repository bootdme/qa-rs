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

CREATE INDEX answers_question_id_idx ON answers(question_id);
