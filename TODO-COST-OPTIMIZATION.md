# TODO: Riduzioni Costi AWS

## Stato Attuale
- **Costo**: ~$305-335/mese
- **Target**: ~$150-200/mese
- **Risparmio già ottenuto**: ~$215/mese (42%)

---

# PARTE 1: Azioni Manuali (Console AWS)

## 1. Compute Savings Plan - ~$60/mese risparmio

**Dove**: AWS Console → Cost Management → Savings Plans
**Impegno**: 1 anno, No Upfront
**Sconto**: 30-35% su Fargate

## 2. RDS Reserved Instance - ~$6/mese risparmio

**Dove**: AWS Console → RDS → Reserved Instances
**Impegno**: 1 anno, No Upfront
**Sconto**: ~35%

---

# PARTE 2: Raccomandazioni AI (Gemini + Codex)

## PRIORITÀ ALTA (Risparmio >$30/mese)

### 2.1 Eliminare NAT Gateway → ~$32-40/mese
**Consenso**: ✅ Entrambi gli agenti concordano

**Azione**: Spostare task Fargate in Public Subnet con IP pubblico
- Mantenere Security Group restrittivi (solo ALB in ingresso)
- DB rimane in Private Subnet (comunicazione via IP privato VPC)
- Aggiungere VPC Endpoints per ECR, S3, Logs (opzionale)

**Rischio**: Medio - richiede modifica networking
**Terraform**:
```hcl
network_configuration {
  subnets          = aws_subnet.public[*].id  # Era: private
  assign_public_ip = true                      # Era: false
  security_groups  = [aws_security_group.ecs_tasks.id]
}
```

### 2.2 Fargate Spot per TUTTI i servizi → ~$25-35/mese
**Consenso**: ✅ Entrambi gli agenti concordano

**Azione**: Abilitare FARGATE_SPOT per api-gateway, event-processor, action-workers
- Sconto ~70% sul compute

**Mitigazioni richieste**:
- Task stateless (già OK)
- Ridurre DEREGISTRATION_DELAY a 15s
- Health check grace period >60s
- Circuit breaker abilitato
- Jobs worker idempotenti

**Rischio**: Basso - già usato per ponder

### 2.3 Migrare a Upstash Redis → ~$10-15/mese
**Consenso**: ✅ Entrambi gli agenti concordano

**Azione**: Sostituire ElastiCache con Upstash (serverless Redis)
- Free tier: 10k comandi/giorno
- Pay-as-you-go: $0.20/100k comandi

**Risparmio**: ~$15/mese (elimina ElastiCache ~$18-20)
**Rischio**: Basso - latenza leggermente superiore

---

## PRIORITÀ MEDIA (Risparmio $5-15/mese)

### 2.4 Secrets Manager → SSM Parameter Store → ~$6/mese
**Consenso**: ✅ Entrambi gli agenti concordano

**Azione**: Migrare segreti non-rotabili su Parameter Store Standard (gratuito)
- Tenere in Secrets Manager solo: RDS password (rotazione automatica)
- Migrare tutto il resto (API keys, JWT secret, OAuth secrets, etc.)

**Terraform**:
```hcl
# Invece di:
# valueFrom = aws_secretsmanager_secret.jwt_secret.arn
# Usare:
valueFrom = "arn:aws:ssm:region:account:parameter/prod/jwt_secret"
```

### 2.5 Ottimizzazioni ALB → ~$3-5/mese
**Consenso**: ✅ Entrambi gli agenti concordano

**Azioni**:
- Health check interval: 30s → 60s
- Healthy threshold: 2 → 3
- Consolidare target groups con path-based routing

---

## PRIORITÀ BASSA (Risparmio <$5/mese)

### 2.6 CloudWatch Logs retention → ~$3-5/mese
Già implementato (7-14 giorni)

### 2.7 ECR Lifecycle Policy → ~$1-2/mese
Tenere solo ultime 5 immagini

### 2.8 Graviton/ARM (futuro) → ~20% extra
Richiede immagini multi-arch

---

## ALTERNATIVE AVANZATE (da valutare)

### Sostituire ALB con API Gateway HTTP API
- **Pro**: $16-20/mese risparmiati (costo base ALB)
- **Contro**: Complessità, VPC Link necessario
- **Quando**: Se traffico <1M req/mese

### Scheduled Scaling (scale-to-zero notturno)
- **Pro**: -50% compute per event-processor/action-workers
- **Contro**: Cold start al mattino
- **Quando**: Se non serve processing 24/7

---

## RIEPILOGO RISPARMIO POTENZIALE

| Azione | Risparmio/Mese | Difficoltà | Priorità |
|--------|----------------|------------|----------|
| Eliminare NAT Gateway | $32-40 | Alta | 🔴 |
| Fargate Spot 100% | $25-35 | Bassa | 🟢 |
| Upstash Redis | $15 | Media | 🟡 |
| SSM Parameter Store | $6 | Bassa | 🟢 |
| ALB ottimizzazioni | $3-5 | Bassa | 🟢 |
| Compute Savings Plan | $60 | Console | 🟡 |
| RDS Reserved | $6 | Console | 🟡 |
| **TOTALE** | **$147-167** | | |

**Nuovo costo stimato**: ~$140-190/mese (era $520/mese)
**Riduzione totale**: ~65-73%

---

## ORDINE DI IMPLEMENTAZIONE CONSIGLIATO

1. ✅ **Fargate Spot 100%** - rischio basso, risparmio alto
2. ✅ **SSM Parameter Store** - facile, risparmio sicuro
3. ✅ **ALB ottimizzazioni** - facile, impatto minimo
4. 🔄 **Upstash Redis** - richiede test integrazione
5. 🔄 **Eliminare NAT Gateway** - richiede test networking
6. ⏳ **Reserved/Savings Plans** - dopo stabilizzazione (1-2 settimane)
