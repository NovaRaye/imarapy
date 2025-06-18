import imarapy


def print_git_style_diff(differences):
    
    for diff in differences:
        print(diff)
        src_pos = diff.source.position
        src_lines = diff.source.lines
        tgt_pos = diff.target.position
        tgt_lines = diff.target.lines
        diff_type = diff.type

        src_count = len(src_lines) if src_lines else 0
        tgt_count = len(tgt_lines) if tgt_lines else 0
        
        if diff_type == imarapy.DELTA_TYPE_INSERT:
            print(f"@@ -{src_pos},0 +{tgt_pos},{tgt_count} @@")
            for line in tgt_lines:
                print(f"+ {line}")
                
        elif diff_type == imarapy.DELTA_TYPE_DELETE:
            print(f"@@ -{src_pos},{src_count} +{tgt_pos},0 @@")
            for line in src_lines:
                print(f"- {line}")
                
        elif diff_type == imarapy.DELTA_TYPE_CHANGE:
            print(f"@@ -{src_pos},{src_count} +{tgt_pos},{tgt_count} @@")
            for line in src_lines:
                print(f"- {line}")
            for line in tgt_lines:
                print(f"+ {line}")
                
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
