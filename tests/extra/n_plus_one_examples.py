"""
Test examples for N+1 detection - both TRUE positives and FALSE positives
"""
import asyncio


# ============================================================================
# FALSE POSITIVES - These should NOT be flagged as N+1
# ============================================================================


async def session_teardown_correct(active_sessions, teardown_func):
    """
    FALSE POSITIVE: This is NOT an N+1 problem.
    Each session must be closed individually - this is necessary iteration.
    Moving close() outside the loop would be incorrect.
    """
    for session_id, session in active_sessions.items():
        if teardown_func is not None:
            try:
                await teardown_func(session)
            except Exception as e:
                print(f"Error tearing down {session_id}: {e}")
        try:
            await session.close()  # Must close each session
        except Exception:
            pass


def process_items(items):
    """
    FALSE POSITIVE: This is NOT an N+1 problem.
    Each item needs to be processed individually.
    """
    results = []
    for item in items:
        result = item.process()  # Different operation per item
        results.append(result)
    return results


async def send_notifications(users):
    """
    FALSE POSITIVE: This is NOT an N+1 problem (if notifications must be individual).
    However, if the API supports batching, then it IS an N+1 problem.
    Context matters!
    """
    for user in users:
        await send_email(user.email, f"Hello {user.name}")


# ============================================================================
# TRUE POSITIVES - These SHOULD be flagged as N+1
# ============================================================================


def load_user_profiles_n_plus_one(user_ids):
    """
    TRUE N+1: Loading users one at a time instead of batching.
    Should use: db.get_users_by_ids(user_ids) for a single query.
    """
    import db
    profiles = []
    for user_id in user_ids:
        profile = db.get_user(user_id)  # Separate query per user
        profiles.append(profile)
    return profiles


def load_user_profiles_fixed(user_ids):
    """
    FIXED: Batch load all users in one query.
    """
    import db
    return db.get_users_by_ids(user_ids)  # Single query


def process_images_n_plus_one(image_paths):
    """
    TRUE N+1: Loading the same model repeatedly in the loop.
    Should load model once before the loop.
    """
    import torch
    results = []
    for path in image_paths:
        model = torch.load("model.pt")  # Loading model N times!
        image = load_image(path)
        result = model(image)
        results.append(result)
    return results


def process_images_fixed(image_paths):
    """
    FIXED: Load model once before the loop.
    """
    import torch
    model = torch.load("model.pt")  # Load once
    results = []
    for path in image_paths:
        image = load_image(path)
        result = model(image)
        results.append(result)
    return results


def read_config_files_n_plus_one(config_names):
    """
    TRUE N+1: Opening the SAME file repeatedly.
    Should read once and cache if accessing multiple times.
    """
    configs = []
    for name in config_names:
        with open("config.json") as f:  # Opening same file N times!
            import json
            data = json.load(f)
            configs.append(data.get(name))
    return configs


def read_config_files_fixed(config_names):
    """
    FIXED: Read file once, then access cached data.
    """
    import json
    with open("config.json") as f:
        data = json.load(f)
    return [data.get(name) for name in config_names]


def fetch_api_data_n_plus_one(item_ids):
    """
    TRUE N+1: Making separate API requests that could be batched.
    Should use batch endpoint: api.get("/items?ids=1,2,3")
    """
    import requests
    items = []
    for item_id in item_ids:
        response = requests.get(f"https://api.example.com/items/{item_id}")
        items.append(response.json())
    return items


def fetch_api_data_fixed(item_ids):
    """
    FIXED: Use batch endpoint to fetch all items at once.
    """
    import requests
    ids_param = ",".join(str(id) for id in item_ids)
    response = requests.get(f"https://api.example.com/items?ids={ids_param}")
    return response.json()


def tokenize_n_plus_one(texts):
    """
    TRUE N+1: Creating tokenizer in every iteration.
    Should create once before the loop.
    """
    from transformers import AutoTokenizer

    results = []
    for text in texts:
        tokenizer = AutoTokenizer.from_pretrained("bert-base-uncased")  # Loading N times!
        tokens = tokenizer(text)
        results.append(tokens)
    return results


def tokenize_fixed(texts):
    """
    FIXED: Load tokenizer once before the loop.
    """
    from transformers import AutoTokenizer

    tokenizer = AutoTokenizer.from_pretrained("bert-base-uncased")  # Load once
    return [tokenizer(text) for text in texts]


async def database_n_plus_one(post_ids):
    """
    TRUE N+1: Classic database N+1 query problem.
    Should use JOIN or WHERE IN clause.
    """
    posts = []
    for post_id in post_ids:
        # Separate query per post
        post = await db.query("SELECT * FROM posts WHERE id = ?", post_id)
        # Then separate query for each post's comments
        comments = await db.query("SELECT * FROM comments WHERE post_id = ?", post_id)
        posts.append({"post": post, "comments": comments})
    return posts


async def database_fixed(post_ids):
    """
    FIXED: Use batch queries with WHERE IN.
    """
    # Single query for all posts
    posts = await db.query(
        "SELECT * FROM posts WHERE id IN (?)",
        post_ids
    )
    # Single query for all comments
    comments = await db.query(
        "SELECT * FROM comments WHERE post_id IN (?)",
        post_ids
    )
    # Group comments by post_id
    comments_by_post = {}
    for comment in comments:
        comments_by_post.setdefault(comment.post_id, []).append(comment)

    return [
        {"post": post, "comments": comments_by_post.get(post.id, [])}
        for post in posts
    ]


def compute_with_cache_miss_n_plus_one(inputs):
    """
    TRUE N+1: Repeated expensive computation that could be cached.
    Same computation with same input is repeated.
    """
    results = []
    for input_val in inputs:
        # Expensive computation repeated even for duplicate inputs
        result = expensive_computation(input_val)
        results.append(result)
    return results


def compute_with_cache_fixed(inputs):
    """
    FIXED: Cache results to avoid recomputation.
    """
    cache = {}
    results = []
    for input_val in inputs:
        if input_val not in cache:
            cache[input_val] = expensive_computation(input_val)
        results.append(cache[input_val])
    return results


# ============================================================================
# Helper functions (stubs)
# ============================================================================

async def send_email(email, message):
    pass

def load_image(path):
    pass

def expensive_computation(x):
    return x ** 2

class db:
    @staticmethod
    def get_user(user_id):
        pass

    @staticmethod
    def get_users_by_ids(user_ids):
        pass

    @staticmethod
    async def query(sql, *args):
        pass
