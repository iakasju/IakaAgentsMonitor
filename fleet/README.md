# fleet/ - demon de machine et CLI `iakafleet`

Piece du produit **IakaAgentsMonitor** (importee d'iakaFleet, historique conserve). Le demon
`iakafleet-agent` tourne en service sur **chaque** machine, **sans GUI** : il lit en lecture seule
les fichiers de session de Claude Code et les fiches d'agents ecrites par le publieur de hooks,
puis publie sur le broker central sous `iaka/<node>/out/...`. La CLI `iakafleet ls|watch|--json`
lit la flotte (machines, sessions, agents).

- Contrat de bus (source unique) : `../specs/contrat-bus-iaka.md`
- Instruction de fusion : `../specs/instructions/fusion-flotte-vue-distante.md`
- Vecteurs de contrat partages : `../contract-vectors/`
- Installation : `docs/installation.md`
- Historique de decision de l'ex-iakaFleet : `specs/PROJET.md` (historique)

```bash
npm install
npm test
npm run lint
```
