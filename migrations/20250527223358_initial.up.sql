CREATE TABLE IF NOT EXISTS public.tokens
(
    user_id character varying NOT NULL,
    expiry_time timestamp with time zone NOT NULL,
    token_type character varying NOT NULL,
    access_token character varying NOT NULL,
    refresh_token character varying NOT NULL,
    CONSTRAINT tokens_pkey PRIMARY KEY (user_id)
);

CREATE TABLE IF NOT EXISTS public.accounts
(
    id character varying NOT NULL,
    user_id character varying NOT NULL,
    description text NOT NULL,
    created timestamp with time zone NOT NULL,
    CONSTRAINT accounts_pkey PRIMARY KEY (id),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES tokens (user_id)
);

CREATE TABLE IF NOT EXISTS public.transactions
(
    id character varying NOT NULL,
    account_id character varying NOT NULL,
    amount bigint NOT NULL,
    currency character varying NOT NULL,
    description text NOT NULL,
    notes text NOT NULL,
    merchant text,
    category text NOT NULL,
    created timestamp with time zone NOT NULL,
    settled timestamp with time zone,
    CONSTRAINT transactions_pkey PRIMARY KEY (id),
    CONSTRAINT fk_account FOREIGN KEY (account_id) REFERENCES accounts (id)
);
