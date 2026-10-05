-- Samakan nama kolom dengan kontrak tim
ALTER TABLE indikator_tesis RENAME COLUMN indikator TO metric_name;

ALTER TABLE snapshot_data RENAME COLUMN indikator TO metric_name;
ALTER TABLE snapshot_data RENAME COLUMN periode   TO period;
ALTER TABLE snapshot_data RENAME COLUMN nilai     TO value;
ALTER TABLE snapshot_data RENAME COLUMN sumber    TO source;
ALTER TABLE snapshot_data ADD COLUMN subsector TEXT;

-- Format nomor WhatsApp sesuai WhatsAppAlertPayload
ALTER TABLE pengguna
    ADD CONSTRAINT nomor_wa_format CHECK (nomor_wa ~ '^[1-9][0-9]{9,14}$');

-- findings dan alerts dibuat ulang (masih kosong)
DROP TABLE alerts;
DROP TABLE findings;

CREATE TABLE findings (
    id                BIGSERIAL PRIMARY KEY,
    ticker            TEXT,
    subsector         TEXT,
    metric_name       TEXT,
    current_value     DOUBLE PRECISION,
    previous_value    DOUBLE PRECISION,
    period            TEXT,
    source            TEXT,
    observed_at       TIMESTAMPTZ,
    confidence_score  DOUBLE PRECISION CHECK (confidence_score BETWEEN 0 AND 1),
    finding_summary   TEXT,
    payload           JSONB NOT NULL,
    status            TEXT NOT NULL DEFAULT 'menunggu'
                      CHECK (status IN ('menunggu', 'lolos', 'ditolak')),
    is_compliant      BOOLEAN,
    evidence_verified BOOLEAN,
    prohibited_words  TEXT[] NOT NULL DEFAULT '{}',
    rejection_reason  TEXT,
    dibuat_pada       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- satu temuan lolos per ticker + metrik + periode
CREATE UNIQUE INDEX findings_lolos_unik
    ON findings (ticker, metric_name, period)
    WHERE status = 'lolos';
CREATE INDEX findings_status ON findings (status);

-- arah dukungan dihitung orchestrator per tesis
CREATE TABLE finding_tesis (
    finding_id    BIGINT NOT NULL REFERENCES findings(id) ON DELETE CASCADE,
    tesis_id      BIGINT NOT NULL REFERENCES tesis(id) ON DELETE CASCADE,
    arah_dukungan TEXT NOT NULL
                  CHECK (arah_dukungan IN ('mendukung', 'melemahkan', 'netral')),
    PRIMARY KEY (finding_id, tesis_id)
);

CREATE TABLE alerts (
    id           BIGSERIAL PRIMARY KEY,
    pengguna_id  BIGINT NOT NULL REFERENCES pengguna(id) ON DELETE CASCADE,
    tesis_id     BIGINT NOT NULL REFERENCES tesis(id) ON DELETE CASCADE,
    finding_id   BIGINT NOT NULL REFERENCES findings(id) ON DELETE CASCADE,
    header       TEXT NOT NULL,
    isi_pesan    TEXT NOT NULL,
    status_kirim TEXT NOT NULL DEFAULT 'menunggu'
                 CHECK (status_kirim IN ('menunggu', 'ditunda', 'terkirim', 'gagal')),
    dikirim_pada TIMESTAMPTZ,
    dibuat_pada  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tesis_id, finding_id)
);
CREATE INDEX alerts_pengguna_waktu ON alerts (pengguna_id, dikirim_pada);