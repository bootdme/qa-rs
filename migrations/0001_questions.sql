CREATE TABLE questions (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL,
    body VARCHAR,
    date_written BIGINT,
    asker_name VARCHAR,
    reported BOOLEAN,
    helpful INTEGER
);

CREATE INDEX questions_product_id_idx ON questions(product_id);
