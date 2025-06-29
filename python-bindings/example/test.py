import asyncio
import python_bindings

MAINNET = "0x534e5f4d41494e"

print(dir(python_bindings))

fork_params = python_bindings.ForkParams(
    "https://starknet-mainnet.public.blastapi.io", 1521205, "0x7aabf76192d3d16fe8bda54c0e7d0a9843c21fe20dd23704366bad38d57dc30"
)

print("Fork parameters:", dir(fork_params))


async def main():
    env_label = await python_bindings.create_environment("test_env", MAINNET, fork_params)
    print("Environment created:", env_label)
    middleware_id = await python_bindings.create_middleware(env_label)
    print("Middleware created:", middleware_id)
    account_id = await python_bindings.create_account(middleware_id, "0x36078334509b514626504edc9fb252328d1a240e4e948bef8d0c08dff45927f")
    print("Account created:", account_id)


with asyncio.Runner() as runner:
    runner.run(main())
