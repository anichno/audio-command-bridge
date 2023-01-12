CREATE TABLE public.users
(
    id serial,
    email text NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE public.auth_tokens
(
    user_id integer NOT NULL,
    token text NOT NULL,
    token_expiration timestamp with time zone NOT NULL,
    refresh_token text NOT NULL,
    refresh_token_expiration timestamp with time zone NOT NULL,
    CONSTRAINT userid UNIQUE (user_id),
    CONSTRAINT user_id FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
        NOT VALID
);

