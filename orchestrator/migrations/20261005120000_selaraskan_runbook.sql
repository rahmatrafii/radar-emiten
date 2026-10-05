-- Penyelarasan skema dengan runbook (Bagian 6)

-- 1. Status tesis: tambah status awal & belum cukup data
ALTER TABLE tesis DROP CONSTRAINT tesis_status_check;
ALTER TABLE tesis ALTER COLUMN status SET DEFAULT 'PENDING_CONFIRMATION';
UPDATE tesis SET status = CASE
    WHEN status = 'menguat' THEN 'MENGUAT'
    WHEN status = 'netral'  THEN 'NETRAL'
    WHEN status = 'melemah' THEN 'MELEMAH'
    ELSE 'BELUM_CUKUP_DATA' END;
ALTER TABLE tesis ADD CONSTRAINT tesis_status_check
    CHECK (status IN ('PENDING_CONFIRMATION', 'MENGUAT', 'NETRAL', 'MELEMAH', 'BELUM_CUKUP_DATA'));

-- 2. Arah metric tesis: naik/turun -> increase/decrease/any
ALTER TABLE indikator_tesis DROP CONSTRAINT indikator_tesis_arah_baik_check;
ALTER TABLE indikator_tesis ALTER COLUMN arah_baik TYPE TEXT;
UPDATE indikator_tesis SET arah_baik = CASE
    WHEN arah_baik = 'naik'  THEN 'increase'
    WHEN arah_baik = 'turun' THEN 'decrease'
    ELSE 'any' END;
ALTER TABLE indikator_tesis ADD CONSTRAINT indikator_tesis_arah_baik_check
    CHECK (arah_baik IN ('increase', 'decrease', 'any'));
ALTER TABLE indikator_tesis RENAME COLUMN arah_baik TO desired_direction;

-- 3. snapshot_data: tambah source ke unique constraint
ALTER TABLE snapshot_data DROP CONSTRAINT snapshot_data_ticker_indikator_periode_key;
ALTER TABLE snapshot_data ADD CONSTRAINT snapshot_data_unique
    UNIQUE (ticker, metric_name, period, source);
CREATE INDEX IF NOT EXISTS snapshot_data_lookup ON snapshot_data (ticker, metric_name, diambil_pada DESC);

-- 4. pesan_masuk: lengkapi field processing agar setara inbound_messages
ALTER TABLE pesan_masuk ADD COLUMN IF NOT EXISTS processing_status TEXT NOT NULL DEFAULT 'received'
    CHECK (processing_status IN ('received','processing','processed','failed','ignored'));
ALTER TABLE pesan_masuk ADD COLUMN IF NOT EXISTS attempt_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pesan_masuk ADD COLUMN IF NOT EXISTS last_error TEXT;
ALTER TABLE pesan_masuk ADD COLUMN IF NOT EXISTS diproses_pada TIMESTAMPTZ;
ALTER TABLE pesan_masuk ADD COLUMN IF NOT EXISTS nomor_pengirim TEXT;
ALTER TABLE pesan_masuk ALTER COLUMN isi DROP NOT NULL;

-- 5. alerts: unique per user+tesis+finding (sudah ada finding_id unique) -> ganti ke kombinasi user/tesis/metric/period
ALTER TABLE alerts DROP CONSTRAINT alerts_tesis_id_finding_id_key;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS metric_name TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS period TEXT;
ALTER TABLE alerts ADD CONSTRAINT alerts_unique_period UNIQUE (pengguna_id, tesis_id, metric_name, period);

-- 6. compliance_audit_logs (tabel baru sesuai Bagian 6.2)
CREATE TABLE IF NOT EXISTS compliance_audit_logs (
    id                  BIGSERIAL PRIMARY KEY,
    finding_id          BIGINT REFERENCES findings(id) ON DELETE SET NULL,
    is_compliant        BOOLEAN NOT NULL,
    evidence_verified   BOOLEAN NOT NULL,
    prohibited_words_detected JSONB NOT NULL DEFAULT '[]',
    rejection_reason    TEXT,
    stage               TEXT NOT NULL,
    details             JSONB,
    dibuat_pada         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS compliance_audit_finding ON compliance_audit_logs (finding_id);

-- 7. penggunaan_kredit: tambah call_reference
ALTER TABLE penggunaan_kredit ADD COLUMN IF NOT EXISTS call_reference TEXT;

-- 8. jejak_agent: perlebar outcome/details
ALTER TABLE jejak_agent ADD COLUMN IF NOT EXISTS outcome TEXT;
ALTER TABLE jejak_agent ADD COLUMN IF NOT EXISTS details JSONB;
ALTER TABLE jejak_agent DROP CONSTRAINT jejak_agent_agent_check;
ALTER TABLE jejak_agent ADD CONSTRAINT jejak_agent_agent_check
    CHECK (agent IN ('scout','analyst','evidence_checker','chief','compliance','presenter','scheduler','dispatcher'));
