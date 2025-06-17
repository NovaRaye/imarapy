import imarapy

def print_git_style_diff(differences):
    
    for diff in differences:
        print(diff)
        src_pos = diff.source.position
        src_rows = diff.source.rows
        tgt_pos = diff.target.position
        tgt_rows = diff.target.rows
        diff_type = diff.type

        # Create a git-style hunk header
        src_count = len(src_rows) if src_rows else 0
        tgt_count = len(tgt_rows) if tgt_rows else 0
        
        if diff_type == "Insert":
            print(f"@@ -{src_pos},0 +{tgt_pos},{tgt_count} @@")
            for row in tgt_rows:
                print(f"+ {row}")
                
        elif diff_type == "Delete":
            print(f"@@ -{src_pos},{src_count} +{tgt_pos},0 @@")
            for row in src_rows:
                print(f"- {row}")
                
        elif diff_type == "Change":
            print(f"@@ -{src_pos},{src_count} +{tgt_pos},{tgt_count} @@")
            for row in src_rows:
                print(f"- {row}")
            for row in tgt_rows:
                print(f"+ {row}")
                
        print()  # Empty line between hunks


def diff_demo():
    
    original_config_str = """\
    {
        "version": "1.0.0",
        "database": {
            "host": "localhost",
            "port": 5432
        },
        "logging": "info"
    }"""
    
    new_config_str = """\
    {
        "version": "1.1.0",
        "database": {
            "host": "db.example.com",
            "port": 5432,
            "username": "admin"
        },
        "logging": "debug",
        "features": {
            "experimental": true
        }
    }"""
    
    original_config = original_config_str.splitlines()
    new_config = new_config_str.splitlines()
    
    differences = imarapy.diff(original_config, new_config, algorithm='histogram')
    print_git_style_diff(differences)
if __name__ == "__main__":
    diff_demo()
