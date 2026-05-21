/* 0043d4ba FUN_0043d4ba */

void __thiscall FUN_0043d4ba(void *this,HWND param_1)

{
  bool bVar1;
  POINT local_48 [5];
  int local_1c;
  undefined1 local_10 [4];
  int local_c;
  int local_8;
  
  *(undefined4 *)((int)this + 0x48) = 100;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x28) + -1; local_8 = local_8 + 1) {
    *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + local_8 * 0x14) =
         *(undefined4 *)((int)this + 0x48);
    *(int *)((int)this + 0x48) = *(int *)((int)this + 0x48) + 1;
    if (*(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)) ==
        *(int *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c))) {
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 0xc + local_8 * 0x14) = 0;
    }
    else {
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 0xc + local_8 * 0x14) = 1;
    }
  }
  *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + (*(int *)((int)this + 0x28) + -1) * 0x14) =
       *(undefined4 *)((int)this + 0x48);
  *(int *)((int)this + 0x48) = *(int *)((int)this + 0x48) + 1;
  bVar1 = true;
  do {
    if (!bVar1) {
      if (*(int *)((int)this + 0x30) == 0) {
        return;
      }
      SendMessageA(param_1,0x8012,(WPARAM)local_10,(LPARAM)&local_c);
      bVar1 = true;
      do {
        if ((*(int *)((int)this + 0x30) < 1) || (!bVar1)) {
          for (local_8 = 0; local_8 < *(int *)((int)this + 0x30); local_8 = local_8 + 1) {
            local_48[0].x = *(LONG *)(*(int *)((int)this + 0x34) + local_8 * 0x14);
            local_48[0].y = *(LONG *)(*(int *)((int)this + 0x34) + 4 + local_8 * 0x14);
            FUN_0043bad3(this,local_48);
            *(undefined4 *)(*(int *)((int)this + 0x34) + 8 + local_8 * 0x14) =
                 *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + local_1c * 0x14);
            if (*(int *)(*(int *)((int)this + 0x34) + 0x10 + local_8 * 0x14) == 0) {
              *(undefined4 *)
               (*(int *)(local_c + *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4)
               + 0xc) = *(undefined4 *)((int)this + 0x4c);
            }
            else {
              *(undefined4 *)
               (*(int *)(local_c + *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4)
               + 0x20) = *(undefined4 *)((int)this + 0x4c);
            }
            if (*(int *)((int)this + 0x38) == -3) {
              if (**(int **)(local_c +
                            *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4) == 1) {
                *(undefined4 *)((int)this + 0x38) =
                     *(undefined4 *)
                      (*(int *)(local_c +
                               *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4) +
                      0x38);
              }
              else if (*(int *)(*(int *)(local_c +
                                        *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14)
                                        * 4) + 0x14) == 1) {
                *(undefined4 *)((int)this + 0x38) =
                     *(undefined4 *)
                      (*(int *)(local_c +
                               *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4) +
                      0x38);
              }
            }
            else {
              *(undefined4 *)
               (*(int *)(local_c + *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4)
               + 0x38) = *(undefined4 *)((int)this + 0x38);
            }
          }
          return;
        }
        bVar1 = false;
        for (local_8 = 0; local_8 < *(int *)((int)this + 0x30); local_8 = local_8 + 1) {
          if (*(int *)(*(int *)(local_c +
                               *(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) * 4) +
                      0x40) != 0) {
            FUN_0043b061(this,local_8);
            bVar1 = true;
            break;
          }
        }
      } while( true );
    }
    bVar1 = false;
    for (local_8 = 1; local_8 < *(int *)((int)this + 0x28) + -1; local_8 = local_8 + 1) {
      if (*(int *)(*(int *)((int)this + 0x2c) + 0xc + local_8 * 0x14) ==
          *(int *)(*(int *)((int)this + 0x2c) + 0xc + (local_8 + -1) * 0x14)) {
        FUN_0043c82b(this,local_8);
        bVar1 = true;
        break;
      }
    }
  } while( true );
}
