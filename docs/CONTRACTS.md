# Data Contracts & Schemas

Dokumen ini mendefinisikan kontrak data resmi (*JSON Schema*) yang digunakan untuk komunikasi antarkomponen dalam **Financial Market Research Agent**:
- MCP Server Rust ↔ Axum Backend
- Axum Backend ↔ Gemini Agentic Orchestrator
- Agent ↔ Compliance Checker ↔ Presenter (WhatsApp Dispatcher)

---

## 1. Schema Utama: `Finding`

Objek `Finding` adalah unit representasi data atomik yang dihasilkan oleh proses pemindaian (*Scout*), analisis fundamental, dan verifikasi kepatuhan (*Compliance*). Setiap *Finding* harus memiliki sumber bukti (*evidence*) yang jelas dan skor keyakinan (*confidence score*).

### JSON Schema (Draft 2020-12)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://radar-emiten.local/schemas/finding.json",
  "title": "Finding",
  "description": "Temuan analisis data pasar emiten yang telah divalidasi dan terikat pada evidensi data historis.",
  "type": "object",
  "required": [
    "ticker",
    "subsector",
    "metric_name",
    "current_value",
    "previous_value",
    "period",
    "source",
    "observed_at",
    "confidence_score",
    "finding_summary"
  ],
  "properties": {
    "ticker": {
      "type": "string",
      "description": "Kode saham emiten Bursa Efek Indonesia (4 huruf kapital).",
      "pattern": "^[A-Z]{4}$",
      "examples": ["BBCA", "TLKM", "ASII"]
    },
    "subsector": {
      "type": "string",
      "description": "Klasifikasi subsektor resmi menurut bursa/Sectors API.",
      "examples": ["banks", "telecommunication", "automobiles-and-components"]
    },
    "metric_name": {
      "type": "string",
      "description": "Nama indikator atau rasio fundamental yang diamati.",
      "examples": ["net_profit_margin", "roe", "revenue_yoy_growth", "per", "pbv", "volume_surge_pct"]
    },
    "current_value": {
      "type": ["number", "string", "null"],
      "description": "Nilai metrik terkini yang diobservasi."
    },
    "previous_value": {
      "type": ["number", "string", "null"],
      "description": "Nilai metrik pada periode perbandingan sebelumnya (bisa bernilai null jika tidak ada data historis)."
    },
    "period": {
      "type": "string",
      "description": "Periode pelaporan keuangan atau rentang observasi.",
      "examples": ["Q3 2024", "FY2023", "TTM", "2024-10-01 to 2024-10-02"]
    },
    "source": {
      "type": "string",
      "description": "Asal endpoint atau dataset Sectors API v2 yang menjadi basis evidensi.",
      "examples": ["Sectors API v2 /v2/company/report/BBCA", "Sectors API v2 /v2/subsector/report/banks"]
    },
    "observed_at": {
      "type": "string",
      "format": "date-time",
      "description": "Waktu ISO 8601 saat data diobservasi/dicatat."
    },
    "confidence_score": {
      "type": "number",
      "minimum": 0.0,
      "maximum": 1.0,
      "description": "Tingkat kepastian data berdasarkan integritas evidensi (0.0 - 1.0)."
    },
    "finding_summary": {
      "type": "string",
      "maxLength": 500,
      "description": "Ringkasan temuan objektif faktual tanpa opini atau arahan investasi (beli/jual/tahan)."
    }
  },
  "additionalProperties": false
}
```

---

## 2. Contoh Data JSON Valid (`Finding`)

Berikut adalah contoh payload konkret yang valid untuk keperluan *unit testing* dan *mocking*:

```json
{
  "ticker": "BBCA",
  "subsector": "banks",
  "metric_name": "net_profit_margin",
  "current_value": 46.85,
  "previous_value": 43.12,
  "period": "Q3 2024",
  "source": "Sectors API v2 /v2/company/report/BBCA",
  "observed_at": "2026-10-02T10:15:30Z",
  "confidence_score": 0.98,
  "finding_summary": "Net Profit Margin (NPM) BBCA pada Q3 2024 tercatat sebesar 46.85%, naik 3.73% poin dibandingkan Q3 2023 sebesar 43.12%, didorong oleh efisiensi beban operasional dan stabilitas pendapatan bunga bersih."
}
```

---

## 3. Schema Kontrak Agen: `ComplianceVerificationResult`

Kontrak yang dihasilkan oleh **Evidence & Compliance Checker Agent** sebelum data diteruskan ke Presenter:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://radar-emiten.local/schemas/compliance_result.json",
  "title": "ComplianceVerificationResult",
  "type": "object",
  "required": [
    "finding_id",
    "is_compliant",
    "prohibited_words_detected",
    "evidence_verified",
    "rejection_reason"
  ],
  "properties": {
    "finding_id": {
      "type": "string"
    },
    "is_compliant": {
      "type": "boolean",
      "description": "True jika temuan bersih dari kata-kata terlarang dan nilai angka sesuai dengan raw data."
    },
    "prohibited_words_detected": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Daftar kata terlarang yang terdeteksi (seperti 'beli', 'target harga', 'cuan')."
    },
    "evidence_verified": {
      "type": "boolean",
      "description": "True jika current_value dan previous_value cocok 1:1 dengan raw snapshot."
    },
    "rejection_reason": {
      "type": ["string", "null"]
    }
  }
}
```

---

## 4. Schema Payload Pesan WhatsApp: `WhatsAppAlertPayload`

Payload yang disusun oleh **Chief Presenter Agent** untuk dikirimkan melalui WhatsApp Cloud API:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://radar-emiten.local/schemas/whatsapp_payload.json",
  "title": "WhatsAppAlertPayload",
  "type": "object",
  "required": ["recipient_number", "header", "body", "disclaimer"],
  "properties": {
    "recipient_number": {
      "type": "string",
      "pattern": "^[1-9][0-9]{9,14}$"
    },
    "header": {
      "type": "string",
      "maxLength": 100
    },
    "body": {
      "type": "string",
      "maxLength": 2048
    },
    "disclaimer": {
      "type": "string",
      "const": "Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. Bukan anjuran investasi atau rekomendasi transaksi."
    }
  }
}
```
