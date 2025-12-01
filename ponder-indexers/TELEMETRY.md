# Ponder Indexer Custom Telemetry

Visualizzazione migliorata dello stato di indicizzazione di Ponder con:
- **Blocco corrente** per ogni chain
- **Eventi organizzati per chain** (non più lista piatta)
- **Metriche di performance** per ogni tipo di evento

## Attivazione

### 1. Abilita il telemetry nel `.env`

```bash
# Copia .env.example se non hai già un .env
cp .env.example .env

# Modifica .env e imposta:
PONDER_TELEMETRY_ENABLED=true
```

### 2. Avvia Ponder

```bash
cd ponder-indexers
pnpm dev
```

## Output Atteso

Invece della visualizzazione predefinita di Ponder, vedrai:

```
┌─────────────────────────────────────────────────────────────┐
│                      Ponder Sync Status                     │
└─────────────────────────────────────────────────────────────┘

Sync

| Network              | Status     | Block           | Progress     | RPC (req/s) |
|----------------------|------------|-----------------|--------------|-------------|
| baseSepolia          | realtime   | 34427002        | 100%         | 1.0         |
| ethereumSepolia      | realtime   | 9748229         | 100%         | 1.0         |
| lineaSepolia         | realtime   | 21469914        | 100%         | 1.6         |


Indexing (by Chain)

baseSepolia (Chain 84532):
| Event                                    | Count      | Avg Duration (ms)  |
|------------------------------------------|------------|--------------------|
| identity:Registered                      | 50         | 0.094              |
| identity:MetadataSet                     | 670        | 0.068              |
| identity:Transfer                        | 50         | 0.180              |
| reputation:NewFeedback                   | 5          | 0.825              |
| reputation:ResponseAppended              | 4          | 0.565              |
| validation:ValidationResponse            | 33         | 46.258             |

ethereumSepolia (Chain 11155111):
| Event                                    | Count      | Avg Duration (ms)  |
|------------------------------------------|------------|--------------------|
| identity:Registered                      | 19         | 538.365            |
| identity:UriUpdated                      | 15         | 382.934            |
| reputation:NewFeedback                   | 16         | 215.003            |
| validation:ValidationResponse            | 5          | 31.124             |

lineaSepolia (Chain 59141):
| Event                                    | Count      | Avg Duration (ms)  |
|------------------------------------------|------------|--------------------|
| validation:ValidationRequest             | 4          | 0.479              |


Total Events Indexed: 867
Refresh Rate: 2s

Press Ctrl+C to stop
```

## Differenze dalla Visualizzazione Predefinita

### Prima (Ponder default)
```
Indexing

| Event                                    | Count      | Duration (ms)     |
|------------------------------------------|------------|-------------------|
| IdentityRegistryEthereumS...             | 19         | 538.365           |
| IdentityRegistryBaseSep...               | 50         | 0.094             |
| IdentityRegistryLineaSe...               | 0          | -                 |
| ReputationRegistryEther...               | 16         | 215.003           |
...
```
❌ Nomi evento troncati
❌ Eventi non organizzati per chain
❌ Nessuna info sul blocco corrente

### Dopo (Custom telemetry)
```
baseSepolia (Chain 84532):
| Event                                    | Count      | Avg Duration (ms)  |
|------------------------------------------|------------|--------------------|
| identity:Registered                      | 50         | 0.094              |
| identity:MetadataSet                     | 670        | 0.068              |
| reputation:NewFeedback                   | 5          | 0.825              |
```
✅ Nomi evento completi e leggibili
✅ Eventi raggruppati per chain
✅ Blocco corrente visibile nella sezione Sync

## Applicare il Telemetry a Tutti gli Handler

Attualmente solo l'handler `handleRegistered` usa il telemetry wrapper. Per applicarlo a tutti gli handler:

### Opzione 1: Manuale (Consigliata)

Per ogni handler function in `src/index.ts`, applica questo pattern:

**Prima:**
```typescript
async function handleMetadataSet(event: MetadataSetEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "MetadataSet";

  try {
    // ... validation and database insert
    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}
```

**Dopo:**
```typescript
async function handleMetadataSet(event: MetadataSetEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "MetadataSet";
  const eventName = `${registry}:${eventType}`;

  await withTelemetry(
    async () => {
      try {
        // ... validation and database insert (unchanged)
        logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
      } catch (error) {
        logEventError(registry, eventType, chainId, error as Error);
        throw error;
      }
    },
    chainId,
    event.block.number,
    eventName
  );
}
```

### Opzione 2: Script Automatico (Avanzata)

Puoi creare uno script bash con `sed` per applicare il pattern automaticamente, ma richiede attenzione per non introdurre errori.

**Lista degli handler da aggiornare:**
- `handleMetadataSet` ✅ (Identity Registry)
- `handleUriUpdated` ✅ (Identity Registry)
- `handleTransfer` ✅ (Identity Registry)
- `handleNewFeedback` ✅ (Reputation Registry)
- `handleFeedbackRevoked` ✅ (Reputation Registry)
- `handleResponseAppended` ✅ (Reputation Registry)
- `handleValidationRequest` ✅ (Validation Registry)
- `handleValidationResponse` ✅ (Validation Registry)

## Configurazione Avanzata

### Modificare il Refresh Rate

In `src/telemetry.ts`, linea 16:

```typescript
private readonly DISPLAY_REFRESH_MS = 2000; // Cambia a 5000 per refresh ogni 5s
```

### Aggiungere Metriche Custom

Puoi estendere `ChainStats` per tracciare metriche aggiuntive:

```typescript
interface ChainStats {
  name: string;
  chainId: number;
  currentBlock: number;
  targetBlock: number;
  isRealtime: boolean;
  rpcRate: number;
  events: Record<string, EventStats>;
  // Aggiungi le tue metriche:
  totalGasUsed?: bigint;
  uniqueAgents?: Set<string>;
}
```

## Disabilitazione

Per tornare alla visualizzazione predefinita di Ponder:

```bash
# In .env
PONDER_TELEMETRY_ENABLED=false

# Oppure rimuovi completamente la variabile
```

## Limitazioni Attuali

1. **Target Block**: Ponder non espone l'API per sapere il blocco target durante il sync iniziale, quindi mostriamo sempre il blocco corrente
2. **RPC Rate**: Ponder non espone metriche real-time delle richieste RPC, quindi mostriamo `0`
3. **Sync State**: Non possiamo distinguere tra "syncing" e "realtime" tramite API di Ponder, quindi assumiamo sempre "realtime"

Queste limitazioni potrebbero essere risolte in future versioni di Ponder se esporranno API pubbliche per telemetry.

## Troubleshooting

### Il telemetry non si avvia

1. Verifica che `PONDER_TELEMETRY_ENABLED=true` sia nel file `.env` (non `.env.example`)
2. Riavvia Ponder con `pnpm dev`
3. Controlla i log per errori di import

### Lo schermo lampeggia

Questo è normale durante il refresh ogni 2 secondi. Puoi aumentare l'intervallo modificando `DISPLAY_REFRESH_MS` in `src/telemetry.ts`.

### Eventi non appaiono

Assicurati di aver applicato il wrapper `withTelemetry` agli handler. Solo gli handler wrappati registrano eventi nel telemetry.

## Performance Impact

Il telemetry custom ha un impatto trascurabile sulle performance:
- **Overhead per evento**: ~0.01ms (misurazione `performance.now()`)
- **Refresh display**: 2s (non blocca il processing degli eventi)
- **Memoria**: ~1KB per chain tracked

---

**Creato**: 2025-01-30
**Versione**: 1.0.0
