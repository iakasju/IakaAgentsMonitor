# Polices embarquées — licence OFL 1.1

Amendement A3 (26/09/2026) : polices de la charte Studio clair embarquées localement dans l'app
(pas de CDN au runtime — l'app est autonome, sans accès réseau). Trois fichiers variables,
sous-ensemble **latin** uniquement (`U+0000-00FF` + ponctuation étendue — couvre les caractères
accentués français : é è à ç î ô û ê â ù…), téléchargés depuis Google Fonts (`fonts.gstatic.com`)
le 26/09/2026.

| Fichier | Famille | Poids couverts | Taille |
|---|---|---|---|
| `manrope-variable-latin.woff2` | Manrope | 600 / 700 / 800 (variable) | 24 Ko |
| `public-sans-variable-latin.woff2` | Public Sans | 400 / 500 / 600 (variable) | 26 Ko |
| `fira-code-variable-latin.woff2` | Fira Code | 400 / 500 (variable) | 36 Ko |

## Licence

Les trois familles sont publiées sous **SIL Open Font License, Version 1.1** (OFL-1.1) :

- **Manrope** — Mikhail Sharanda, dépôt [github.com/sharanda/manrope](https://github.com/sharanda/manrope).
- **Public Sans** — USWDS / 18F (General Services Administration), dépôt
  [github.com/uswds/public-sans](https://github.com/uswds/public-sans).
- **Fira Code** — Nikita Prokopov, basé sur Fira Mono (Mozilla/Carrois Type Design), dépôt
  [github.com/tonsky/FiraCode](https://github.com/tonsky/FiraCode).

L'OFL autorise l'usage, l'étude, la modification et la redistribution — y compris embarquée dans
un logiciel — à condition de conserver la licence et de ne pas vendre la police seule sous son
nom réservé. Aucune modification n'a été apportée aux polices ; seul un sous-ensemble de
caractères (latin) a été servi par Google Fonts, pratique courante et couverte par la licence.
Texte complet de l'OFL 1.1 : <https://openfontlicense.org/open-font-license-official-text/>.

## Provenance technique

Sous-ensemble extrait de la réponse de
`https://fonts.googleapis.com/css2?family=Manrope:wght@600;700;800&family=Public+Sans:wght@400;500;600&family=Fira+Code:wght@400;500&display=swap`
(en-tête `User-Agent` moderne pour obtenir du WOFF2), en ne conservant que le bloc `@font-face`
dont l'`unicode-range` couvre `U+0000-00FF` (sous-ensemble « latin »). Un seul fichier variable
par famille dessert tous les poids demandés (Google sert les trois familles comme polices
variables ; chaque poids déclaré dans `styles.css` pointe vers le même fichier, le moteur de
police du système en extrait l'instance statique correspondante).
