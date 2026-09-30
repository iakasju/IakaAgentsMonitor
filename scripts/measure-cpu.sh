#!/usr/bin/env bash
# Protocole de mesure CPU/mémoire de D7 (specs/instructions/ce-poste-vue-locale-lisibilite.md).
# À lancer par le DÉCIDEUR sur un BUILD DE PUBLICATION, au repos (aucun agent actif), fenêtre
# fermée puis ouverte :
#
#   npm run tauri build
#   open src-tauri/target/release/bundle/macos/IakaAgentsMonitor.app     # (ou lancer le binaire)
#   IAKAAM_TRACE=1 src-tauri/target/release/iakaagentsmonitor 2> tick.log &   # durée de tick
#   scripts/measure-cpu.sh                      # 600 s, fenêtre fermée ; recommencer fenêtre ouverte
#
# Budget D7 : CPU MOYEN ≤ 1 %, AUCUN échantillon > 10 % (hors les 5 premières secondes) ;
# durée d'un tick ≤ 50 ms au 95e centile (voir la sortie de --tick-log).
#
# Échantillonnage chaque seconde. Deux mesures : `ps -o %cpu` (celle du cadrage ; sous macOS c'est une
# moyenne glissante qui lisse les pics) ET le delta de temps CPU cumulé `ps -o cputime` (CPU réel de
# chaque seconde) — c'est ce dernier qui sert au verdict « aucun pic > 10 % ».
set -euo pipefail

PID=""
SECONDS_TOTAL=600
WARMUP=5
TICK_LOG=""
while [ $# -gt 0 ]; do
  case "$1" in
    --pid) PID="$2"; shift 2 ;;
    --seconds) SECONDS_TOTAL="$2"; shift 2 ;;
    --warmup) WARMUP="$2"; shift 2 ;;
    --tick-log) TICK_LOG="$2"; shift 2 ;;
    *) echo "usage: $0 [--pid N] [--seconds 600] [--warmup 5] [--tick-log tick.log]" >&2; exit 2 ;;
  esac
done

if [ -z "$PID" ]; then
  PID="$(pgrep -x iakaagentsmonitor | head -n1 || true)"
fi
if [ -z "$PID" ] || ! kill -0 "$PID" 2>/dev/null; then
  echo "processus iakaagentsmonitor introuvable (lancer l'app, ou --pid N)" >&2
  exit 1
fi

# cputime "M:SS.cc" ou "H:MM:SS" -> secondes (flottant)
cpu_seconds() {
  ps -o cputime= -p "$1" | tr -d ' ' | awk -F: '{ if (NF==3) print $1*3600+$2*60+$3; else if (NF==2) print $1*60+$2; else print $1 }'
}

echo "pid $PID, ${SECONDS_TOTAL} s, échauffement ${WARMUP} s" >&2
prev="$(cpu_seconds "$PID")"
: > /dev/null
samples=""
for i in $(seq 1 "$SECONDS_TOTAL"); do
  sleep 1
  kill -0 "$PID" 2>/dev/null || { echo "le processus s'est arrêté à ${i} s" >&2; break; }
  cur="$(cpu_seconds "$PID")"
  psline="$(ps -o %cpu=,rss= -p "$PID" | awk '{print $1, $2}')"
  samples="${samples}${i} $(awk -v a="$cur" -v b="$prev" 'BEGIN{printf "%.2f", (a-b)*100}') ${psline}
"
  prev="$cur"
done

printf '%s' "$samples" | awk -v warm="$WARMUP" '
  $1 > warm { n++; s += $2; if ($2 > mx) mx = $2; if ($2 > 10) over++; ps += $3; if ($4 > rss) rss = $4 }
  END {
    if (n == 0) { print "aucun échantillon"; exit 1 }
    printf "échantillons : %d (hors %d premières s)\n", n, warm
    printf "CPU réel (delta cputime) : moyenne %.2f %% · max %.2f %% · échantillons > 10 %% : %d\n", s/n, mx, over
    printf "ps %%cpu (lissé)          : moyenne %.2f %%\n", ps/n
    printf "RSS max                  : %.1f Mo\n", rss/1024
    printf "verdict D7 (moyenne <= 1 %% et aucun pic > 10 %%) : %s\n", (s/n <= 1 && over == 0) ? "TENU" : "NON TENU"
  }'

if [ -n "$TICK_LOG" ] && [ -f "$TICK_LOG" ]; then
  grep -o 'tick [0-9.]* ms' "$TICK_LOG" | awk '{print $2}' | sort -n | awk '
    { a[NR] = $1 } END { if (NR) printf "durée de tick : %d ticks · p50 %.2f ms · p95 %.2f ms · max %.2f ms (budget p95 <= 50 ms)\n", NR, a[int((NR+1)/2)], a[int(NR*0.95 + 0.5) > NR ? NR : int(NR*0.95 + 0.5)], a[NR] }'
fi
