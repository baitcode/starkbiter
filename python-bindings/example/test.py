import json
import asyncio
import python_bindings
from nethermind.starknet_abi.core import StarknetAbi


MAINNET = "0x534e5f4d41494e"

fork_params = python_bindings.ForkParams(
    "https://starknet-mainnet.public.blastapi.io", 1521205, "0x7aabf76192d3d16fe8bda54c0e7d0a9843c21fe20dd23704366bad38d57dc30"
)

with open("./tmp/ERC20.class.json") as f:
    raw_abi = json.load(f)

abi = StarknetAbi.from_json(raw_abi["abi"], "starknet_eth", b"")


ETH_ERC20_MAINNET = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"
EKUBO_CORE_MAINNET = "0x00000005dd3d2f4429af886cd1a3b08289dbcea99a294197e9eb43b0e0325b4b"

balance_of_function = abi.functions["balanceOf"]

balance_of_calldata = balance_of_function.encode({
    "account": "0x00000005dd3d2f4429af886cd1a3b08289dbcea99a294197e9eb43b0e0325b4b"
})

# print("Function:", balance_of_function)
# print("BalanceOf:", dir(balance_of_calldata))

python_bindings.set_tracing()


async def main():
    print("Starting Starknet Python Bindings Example...", fork_params)
    env_label = await python_bindings.create_environment("test_env", MAINNET, fork_params)
    print("Environment created:", env_label)
    middleware_id = await python_bindings.create_middleware(env_label)
    print("Middleware created:", middleware_id)
    # # account_id = await python_bindings.create_account(middleware_id, "0x36078334509b514626504edc9fb252328d1a240e4e948bef8d0c08dff45927f")
    # # print("Account created:", account_id)

    # print("asdasd")
    # print(balance_of_function.signature.hex())

    call = python_bindings.Call(
        ETH_ERC20_MAINNET,
        balance_of_function.signature.hex(),
        balance_of_calldata,
    )
    res = await python_bindings.call(middleware_id, call, python_bindings.BlockId.from_tag("latest"))
    print("Call result:", res)


with asyncio.Runner() as runner:
    runner.run(main())
