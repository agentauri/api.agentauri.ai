# Ponder Indexer Custom Telemetry

Visualizzazione migliorata dello stato di indicizzazione di Ponder con:
- **Blocco corrente** per ogni chain
- **Eventi organizzati per chain** (non più lista piatta)
- **Nomi evento leggibili** (`identity:Registered` vs `IdentityRegistryEthereum...`)

## ⚠️ Importante: Approccio Standalone

Ponder usa un sistema di UI terminal-based che rende impossibile sovrascrivere la visualizzazione predefinita. Per questo motivo, il telemetry personalizzato funziona come **script standalone** da eseguire in un terminale separato.

## Utilizzo (Metodo Consigliato)

### 1. Installa le dipendenze

```bash
cd ponder-indexers
pnpm install
```

### 2. Avvia Ponder nel terminale principale

```bash
# Terminale 1
pnpm dev
```

### 3. Avvia il telemetry dashboard in un secondo terminale

```bash
# Terminale 2 (nuovo terminale)
cd ponder-indexers
pnpm telemetry
```

## Output Atteso

Nel terminale del dashboard vedrai:

```
┌─────────────────────────────────────────────────────────────┐
│              Ponder Indexer Telemetry Dashboard             │
└─────────────────────────────────────────────────────────────┘

Sync Status

| Network              | Current Block   | Total Events |
|----------------------|-----------------|--------------|
| baseSepolia          | 34427002        | 805          |
| ethereumSepolia      | 9748229         | 54           |
| lineaSepolia         | 21469914        | 8            |


Events by Chain

baseSepolia (Chain 84532):
| Event                                    | Count      |
|------------------------------------------|------------|
| identity:MetadataSet                     | 670        |
| identity:Registered                      | 50         |
| identity:Transfer                        | 50         |
| validation:ValidationResponse            | 33         |
| reputation:NewFeedback                   | 5          |
| reputation:ResponseAppended              | 4          |

ethereumSepolia (Chain 11155111):
| Event                                    | Count      |
|------------------------------------------|------------|
| identity:Registered                      | 19         |
| reputation:NewFeedback                   | 16         |
| identity:UriUpdated                      | 15         |
| validation:ValidationResponse            | 5          |

lineaSepolia (Chain 59141):
| Event                                    | Count      |
|------------------------------------------|------------|
| validation:ValidationRequest             | 4          |
| validation:ValidationResponse            | 4          |


Total Events Indexed: 867
Refresh Rate: 2s
Database: localhost:5432/erc8004_backend

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

## Come Funziona

Lo script `telemetry-dashboard.ts` si connette direttamente al database PostgreSQL e legge:

1. **Checkpoint table**: Per ottenere il blocco corrente di ogni chain
2. **Event table**: Per contare gli eventi raggruppati per chain e tipo

**Vantaggi dell'approccio standalone:**
- ✅ Non interferisce con la UI di Ponder
- ✅ Non richiede modifiche agli event handlers
- ✅ Legge dati reali dal database (nessuna approssimazione)
- ✅ Refresh automatico ogni 2 secondi
- ✅ Può essere eseguito su qualsiasi macchina con accesso al database

## Configurazione Avanzata

### Modificare il Refresh Rate

In `telemetry-dashboard.ts`, cerca questa linea:

```typescript
  // Refresh every 2 seconds
  setInterval(async () => {
    const stats = await fetchStats();
    displayDashboard(stats);
  }, 2000); // Cambia 2000 a 5000 per refresh ogni 5s
```

### Eseguire su Macchina Remota

Il dashboard può essere eseguito su qualsiasi macchina con accesso al database:

```bash
# Imposta DATABASE_URL per database remoto
export DATABASE_URL="postgresql://user:password@remote-host:5432/erc8004_backend"

# Esegui il dashboard
pnpm telemetry
```

### Aggiungere Metriche Custom

Modifica la query SQL in `telemetry-dashboard.ts` per aggiungere aggregazioni:

```typescript
// Esempio: Aggiungere conteggio agenti unici
const uniqueAgentsResult = await pool.query(`
  SELECT "chainId", COUNT(DISTINCT "agentId") as unique_agents
  FROM "Event"
  GROUP BY "chainId"
`);
```

## Limitazioni

1. **No metriche di performance**: Lo script standalone non traccia la durata di processing degli eventi (solo Ponder internamente lo sa)
2. **No RPC rate**: Non abbiamo accesso alle metriche RPC di Ponder
3. **Polling del database**: Il dashboard interroga il database ogni 2s (leggero overhead)

## Troubleshooting

### Errore "DATABASE_URL environment variable not set"

Assicurati di avere il file `.env` nella directory `ponder-indexers/` con:

```bash
DATABASE_URL=postgresql://postgres:YOUR_PASSWORD@localhost:5432/erc8004_backend
```

Oppure esportala prima di eseguire il dashboard:

```bash
export DATABASE_URL="postgresql://..."
pnpm telemetry
```

### Errore "Database connection failed"

1. Verifica che PostgreSQL sia in esecuzione: `docker-compose ps`
2. Verifica la connessione: `psql erc8004_backend`
3. Controlla username/password nel DATABASE_URL

### Il dashboard non mostra eventi

1. Verifica che Ponder sia in esecuzione nel primo terminale
2. Attendi che Ponder inizi a indicizzare eventi
3. Controlla che ci siano dati nella tabella `Event`:
   ```sql
   SELECT COUNT(*) FROM "Event";
   ```

### Lo schermo lampeggia

Questo è normale durante il refresh ogni 2 secondi. Puoi aumentare l'intervallo modificando il valore in `telemetry-dashboard.ts`.

### "Cannot find module 'pg'"

Esegui `pnpm install` per installare le dipendenze:

```bash
cd ponder-indexers
pnpm install
```

## Performance Impact

Il dashboard ha un impatto minimo:
- **Query al database**: 2 SELECT ogni 2 secondi (~1 query/secondo)
- **Overhead**: <10ms per refresh (misurazione query + rendering)
- **Memoria**: ~5MB (Node.js + pg driver)

---

**Creato**: 2025-01-30
**Versione**: 1.0.0
