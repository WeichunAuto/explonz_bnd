# 同时启动两个服务（需要 tmux）
dev:
    tmux new-session -d -s explonz -n api 'cargo run -p explonz_bnd'
    tmux new-window -t explonz -n admin 'cd explonz_admin && cargo leptos watch'
    tmux attach -t explonz

# 只启动主 API
api:
    cargo run -p explonz_bnd

# 只启动 Admin
admin:
    cd explonz_admin && cargo leptos watch

# 停止 tmux session
stop:
    tmux kill-session -t explonz
