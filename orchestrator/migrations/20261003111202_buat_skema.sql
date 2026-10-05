CREATE TABLE pengguna (
    id           BIGSERIAL PRIMARY KEY,
    nomor_wa     TEXT NOT NULL UNIQUE,
    opt_in       BOOLEAN NOT NULL DEFAULT FALSE,
    jeda_sampai  TIMESTAMPTZ,
    dibuat_pada  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tesis (
    id           BIGSERIAL PRIMARY KEY,
    pengguna_id  BIGINT NOT NULL REFERENCES pengguna(id) ON DELETE CASCADE,
    teks_asli    TEXT NOT NULL,
    ticker       TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'netral'
                 CHECK (status IN ('menguat', 'netral', 'melemah')),
    aktif        BOOLEAN NOT NULL DEFAULT FALSE,
    dibuat_pada  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE indikator_tesis (
    id         BIGSERIAL PRIMARY KEY,
    tesis_id   BIGINT NOT NULL REFERENCES tesis(id) ON DELETE CASCADE,
    indikator  TEXT NOT NULL,
    arah_baik  TEXT NOT NULL CHECK (arah_baik IN ('naik', 'turun')),
    UNIQUE (tesis_id, indikator)
);

CREATE TABLE snapshot_data (
    id           BIGSERIAL PRIMARY KEY,
    ticker       TEXT NOT NULL,
    indikator    TEXT NOT NULL,
    periode      TEXT NOT NULL,
    nilai        DOUBLE PRECISION NOT NULL,
    sumber       TEXT NOT NULL,
    diambil_pada TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (ticker, indikator, periode)
);

CREATE TABLE findings (
    id                BIGSERIAL PRIMARY KEY,
    tesis_id          BIGINT NOT NULL REFERENCES tesis(id) ON DELETE CASCADE,
    indikator         TEXT NOT NULL,
    arah_dukungan     TEXT CHECK (arah_dukungan IN ('mendukung', 'melemahkan', 'netral')),
    nilai_sebelum     DOUBLE PRECISION,
    nilai_sekarang    DOUBLE PRECISION,
    periode           TEXT,
    sumber            TEXT,
    tingkat_keyakinan TEXT CHECK (tingkat_keyakinan IN ('tinggi', 'sedang', 'rendah')),
    status            TEXT NOT NULL CHECK (status IN ('lolos', 'ditolak')),
    alasan_tolak      TEXT,
    dibuat_pada       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- satu temuan lolos per tesis + indikator + periode (pencegah duplikat, F7)
CREATE UNIQUE INDEX findings_lolos_unik
    ON findings (tesis_id, indikator, periode)
    WHERE status = 'lolos';

CREATE TABLE alerts (
    id           BIGSERIAL PRIMARY KEY,
    pengguna_id  BIGINT NOT NULL REFERENCES pengguna(id) ON DELETE CASCADE,
    tesis_id     BIGINT NOT NULL REFERENCES tesis(id) ON DELETE CASCADE,
    finding_id   BIGINT NOT NULL UNIQUE REFERENCES findings(id) ON DELETE CASCADE,
    isi_pesan    TEXT NOT NULL,
    status_kirim TEXT NOT NULL DEFAULT 'menunggu'
                 CHECK (status_kirim IN ('menunggu', 'ditunda', 'terkirim', 'gagal')),
    dikirim_pada TIMESTAMPTZ,
    dibuat_pada  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pesan_masuk (
    id_pesan_wa  TEXT PRIMARY KEY,
    pengguna_id  BIGINT REFERENCES pengguna(id) ON DELETE SET NULL,
    isi          TEXT NOT NULL,
    diterima_pada TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE penggunaan_kredit (
    id          BIGSERIAL PRIMARY KEY,
    endpoint    TEXT NOT NULL,
    kredit      INTEGER NOT NULL DEFAULT 0,
    dari_cache  BOOLEAN NOT NULL DEFAULT FALSE,
    dicatat_pada TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE jejak_agent (
    id           BIGSERIAL PRIMARY KEY,
    agent        TEXT NOT NULL CHECK (agent IN ('scout', 'analyst', 'evidence_checker', 'chief')),
    aksi         TEXT NOT NULL,
    hasil_singkat TEXT,
    ticker       TEXT,
    dicatat_pada TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX jejak_agent_waktu ON jejak_agent (dicatat_pada DESC);
CREATE INDEX findings_tesis ON findings (tesis_id);