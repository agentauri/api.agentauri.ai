# Ponder Telemetry Dashboard - Quick Start

## 🚀 Come Usarlo

Apri **2 terminali** nella directory `ponder-indexers/`:

### Terminale 1 - Ponder (indexer)
```bash
cd ponder-indexers
pnpm dev
```
*(Continua a mostrare l'output nativo di Ponder)*

### Terminale 2 - Telemetry Dashboard
```bash
cd ponder-indexers
pnpm telemetry
```
*(Mostra visualizzazione migliorata con eventi raggruppati per chain)*

## 📊 Cosa Vedrai

Nel **Terminale 2** vedrai:

```
┌─────────────────────────────────────────────────────────────┐
│              Ponder Indexer Telemetry Dashboard             │
│                    Last Update: 10:28:26 PM        │  ← SI AGGIORNA!
└─────────────────────────────────────────────────────────────┘

Sync Status

| Network              | Current Block   | Total Events |
|----------------------|-----------------|--------------|
| baseSepolia          | 34422758        | 836          |
| ethereumSepolia      | 9747636         | 129          |  ← Appare qui!
| lineaSepolia         | 21469914        | 0            |

Events by Chain

baseSepolia (Chain 84532):
| Event                    | Count |
|--------------------------|-------|
| identity:MetadataSet     | 670   |  ← Nomi leggibili
| identity:Registered      | 50    |  ← Raggruppati per chain
| reputation:NewFeedback   | 5     |

ethereumSepolia (Chain 11155111):
| Event                    | Count |
|--------------------------|-------|
| identity:MetadataSet     | 50    |
| identity:Registered      | 19    |
| reputation:NewFeedback   | 16    |
```

## ❓ FAQ

### "I numeri non si aggiornano!"

✅ **Questo è NORMALE**. Significa che Ponder è **completamente sincronizzato** e sta aspettando nuovi blocchi sulla blockchain.

**Come verificare che funziona:**
- Guarda il **timestamp** ("Last Update: HH:MM:SS") - si aggiorna ogni 2 secondi
- Se cambia, il dashboard funziona correttamente!

### "Non vedo ethereumSepolia!"

Controlla se:
1. ✅ Hai scrollato verso l'alto? (Il terminale potrebbe mostrare solo la parte finale)
2. ✅ Ponder ha finito di indicizzare? (Attendi qualche secondo e controlla nel Terminale 1)
3. ✅ La chain è configurata? (Verifica `.env` per `ETHEREUM_SEPOLIA_RPC_ALCHEMY`)

**Debug rapido:**
```bash
# Controlla quali chain sono nel database
PGPASSWORD="..." psql -d erc8004_backend -c "SELECT * FROM \"Checkpoint\";"
```

### "Vedo solo baseSepolia con 836 eventi"

Se vedi **solo** baseSepolia, probabilmente:
- Ponder è ancora in fase di sync iniziale di altre chain
- Oppure le altre chain non hanno eventi ancora (normale per testnet)

**Verifica:**
1. Guarda i log di Ponder (Terminale 1)
2. Cerca messaggi tipo "Sync: 100%" o "Real-time mode"

### "Il terminale lampeggia troppo"

Puoi modificare il refresh rate in `ponder-indexers/telemetry-dashboard.ts`:

```typescript
// Cambia da 2000 (2s) a 5000 (5s)
setInterval(async () => {
  const stats = await fetchStats();
  displayDashboard(stats);
}, 5000); // ← Qui
```

## 🎯 Differenza dal Output Nativo di Ponder

### Prima (Ponder nativo - Terminale 1):
```
Indexing
│ IdentityRegistryEthereumS... │ 19  │ 538.365 │  ← Troncato
│ ReputationRegistryBa...      │ 16  │ 215.003 │  ← Non raggruppato
```

### Dopo (Telemetry Dashboard - Terminale 2):
```
baseSepolia (Chain 84532):
| identity:Registered      | 50  |  ← Nome completo
| reputation:NewFeedback   | 5   |  ← Raggruppato per chain

ethereumSepolia (Chain 11155111):
| identity:Registered      | 19  |
| reputation:NewFeedback   | 16  |
```

## 📚 Documentazione Completa

Vedi `ponder-indexers/TELEMETRY.md` per:
- Configurazione avanzata
- Troubleshooting dettagliato
- Modificare query SQL personalizzate

---

**Pro tip**: Usa `tmux` o `screen` per mantenere entrambi i terminali visibili contemporaneamente!

```bash
# Con tmux (se installato)
tmux
# Ctrl+B poi "  (split orizzontale)
# Terminale sopra: pnpm dev
# Terminale sotto: pnpm telemetry
```
