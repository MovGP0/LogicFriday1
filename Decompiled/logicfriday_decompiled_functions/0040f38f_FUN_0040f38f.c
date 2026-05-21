/* 0040f38f FUN_0040f38f */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0040f38f(void *this,int param_1)

{
  undefined4 uVar1;
  uint unaff_retaddr;
  undefined4 *local_60c;
  undefined1 local_604 [12];
  undefined4 local_5f8;
  undefined4 local_5f4;
  undefined1 local_5dc [12];
  undefined4 local_5d0;
  undefined4 local_5cc;
  undefined1 local_5b4 [8];
  undefined4 local_5ac;
  char *local_5a0;
  undefined1 local_58c [8];
  undefined4 local_584;
  uint *local_578;
  undefined1 local_564 [8];
  undefined4 local_55c;
  char *local_550;
  undefined1 local_53c [8];
  undefined4 local_534;
  uint *local_528;
  undefined1 local_514 [8];
  undefined4 local_50c;
  uint *local_500;
  undefined1 local_4ec [8];
  undefined4 local_4e4;
  uint *local_4d8;
  undefined1 local_4c4 [8];
  undefined4 local_4bc;
  uint *local_4b0;
  undefined1 local_49c [8];
  undefined4 local_494;
  uint *local_488;
  undefined1 local_474 [8];
  undefined4 local_46c;
  uint *local_460;
  int local_44c;
  int local_448;
  undefined4 local_444;
  int local_440;
  int local_438;
  int local_434;
  WPARAM local_430;
  uint local_42c [8];
  uint local_40c [257];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_44c = 0;
  *(undefined4 *)((int)this + 0x28) = 4;
  local_448 = 0;
  local_430 = 0;
  do {
    if (*(int *)((int)this + 0x50) <= (int)local_430) {
LAB_0040f436:
      if (local_448 == 0) {
        uVar1 = 0;
      }
      else {
        local_434 = param_1;
        *(undefined4 *)((int)this + 0x30) = 0;
        *(undefined4 *)((int)this + 0x40) = 9;
        *(undefined4 *)((int)this + 0x28) = 5;
        FUN_0043ed39((char *)local_40c,&DAT_0044b960);
        local_46c = 1;
        local_460 = local_40c;
        SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_474
                    );
        FUN_0043ed39((char *)local_40c,&DAT_0044b960);
        local_494 = 2;
        local_488 = local_40c;
        SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_49c
                    );
        FUN_0043ed39((char *)local_40c,&DAT_0044b960);
        for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
          FUN_0043ed39((char *)local_42c,&DAT_0044b958);
          FUN_0043ebe0(local_40c,local_42c);
        }
        local_4bc = 3;
        local_4b0 = local_40c;
        SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_4c4
                    );
        FUN_0043ed39((char *)local_40c,&DAT_0044b960);
        for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
          FUN_0043ed39((char *)local_42c,&DAT_0044b958);
          FUN_0043ebe0(local_40c,local_42c);
        }
        local_4e4 = 4;
        local_4d8 = local_40c;
        SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_4ec
                    );
        FUN_0043ed39((char *)local_40c,&DAT_0044b960);
        for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
          FUN_0043ed39((char *)local_42c,&DAT_0044b958);
          FUN_0043ebe0(local_40c,local_42c);
        }
        local_50c = 5;
        local_500 = local_40c;
        SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_514
                    );
        if (*(int *)(param_1 + 0x23c) == 0) {
          local_55c = 6;
          local_550 = "Unminimized";
          SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),
                       (LPARAM)local_564);
        }
        else {
          FUN_0043ed39((char *)local_40c,&DAT_0044b960);
          local_534 = 6;
          local_528 = local_40c;
          SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),
                       (LPARAM)local_53c);
        }
        if (*(int *)(param_1 + 0x1650) == 0) {
          local_5ac = 7;
          local_5a0 = "Not mapped";
          SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),
                       (LPARAM)local_5b4);
        }
        else {
          local_44c = 0;
          for (local_430 = *(int *)(param_1 + 0x1654); (int)local_430 < *(int *)(param_1 + 0x1658);
              local_430 = local_430 + 1) {
            if (((*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_430 * 0xfc) == 0) &&
                (*(int *)(local_430 * 0xfc + *(int *)(param_1 + 0x3a4)) != 10)) &&
               (*(int *)(local_430 * 0xfc + *(int *)(param_1 + 0x3a4)) != 0xb)) {
              local_44c = local_44c + 1;
            }
          }
          FUN_0043ed39((char *)local_40c,&DAT_0044b960);
          local_584 = 7;
          local_578 = local_40c;
          SendMessageA(*(HWND *)((int)this + 4),0x102e,*(WPARAM *)((int)this + 0x2c),
                       (LPARAM)local_58c);
        }
        for (local_430 = 0; (int)local_430 < *(int *)((int)this + 0x50); local_430 = local_430 + 1)
        {
          local_5cc = 2;
          local_5d0 = 0;
          SendMessageA(*(HWND *)((int)this + 4),0x102b,local_430,(LPARAM)local_5dc);
        }
        local_5f4 = 2;
        local_5f8 = 2;
        SendMessageA(*(HWND *)((int)this + 4),0x102b,*(WPARAM *)((int)this + 0x2c),(LPARAM)local_604
                    );
        if (&stack0x00000000 == (undefined1 *)0x444) {
          local_60c = (undefined4 *)0x0;
        }
        else {
          local_444 = 0;
          local_60c = &local_444;
        }
        SendMessageA(*(HWND *)((int)this + 4),0x100e,0,(LPARAM)local_60c);
        SendMessageA(*(HWND *)((int)this + 4),0x1014,0,local_438 - local_440);
        uVar1 = 1;
      }
      return uVar1;
    }
    *(WPARAM *)((int)this + 0x2c) = local_430;
    SendMessageA(*(HWND *)((int)this + 4),0x1005,0,(int)this + 0x28);
    if (*(int *)((int)this + 0x48) == param_1) {
      local_448 = 1;
      goto LAB_0040f436;
    }
    local_430 = local_430 + 1;
  } while( true );
}
