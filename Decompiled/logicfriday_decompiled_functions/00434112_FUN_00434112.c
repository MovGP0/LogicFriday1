/* 00434112 FUN_00434112 */

int __fastcall FUN_00434112(uint *param_1)

{
  size_t sVar1;
  int iVar2;
  void *pvVar3;
  int local_24;
  uint local_18;
  int local_10;
  uint local_c;
  
  local_18 = 0;
  local_c = 0;
  local_10 = 0;
  do {
    if ((int)param_1[0x5b1] <= local_10) {
      if ((int)local_18 < 2) {
        iVar2 = 0x3ed0000;
      }
      else if ((int)local_18 < 0x11) {
        if (local_c == 0) {
          iVar2 = 0x3ef0000;
        }
        else if ((int)local_c < 0x11) {
          iVar2 = FUN_00434433(param_1);
          if (iVar2 == 0) {
            FUN_00436925(param_1,(undefined4 *)0x0,0);
            param_1[0x31] = local_18;
            param_1[0x32] = local_c;
            *param_1 = 1 << ((byte)local_18 & 0x1f);
            for (local_10 = 0; local_10 < (int)param_1[0x32]; local_10 = local_10 + 1) {
              pvVar3 = _realloc((void *)param_1[local_10 + 0x21],*param_1 << 2);
              param_1[local_10 + 0x21] = (uint)pvVar3;
              param_1[local_10 + 0x11] = 0;
              param_1[local_10 + 1] = 0;
              _memset((void *)param_1[local_10 + 0x21],0,*param_1 << 2);
            }
            iVar2 = FUN_00435399();
            if (((iVar2 == 0) && (iVar2 = FUN_004358b3(param_1), iVar2 == 0)) &&
               (iVar2 = FUN_0042093b(param_1,1,"Entered by gate diagram:"), iVar2 == 0)) {
              iVar2 = 0;
            }
          }
        }
        else {
          iVar2 = 0x3f00000;
        }
      }
      else {
        iVar2 = 0x3ee0000;
      }
      return iVar2;
    }
    if (*(int *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0x48) == 0) {
      *(undefined4 *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0xd8) = 0;
      iVar2 = **(int **)(param_1[0x5b3] + local_10 * 4);
      if (((*(int *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0xe0) == -3) && (iVar2 != 9)) &&
         (iVar2 != 8)) {
        return local_10 + 0x3e90000;
      }
      for (local_24 = 0; local_24 < *(int *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0x18);
          local_24 = local_24 + 1) {
        if (*(int *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0xe4 + local_24 * 4) == -3) {
          return local_10 + 0x3ea0000;
        }
      }
      if (**(int **)(param_1[0x5b3] + local_10 * 4) == 8) {
        FUN_0043ebd0((uint *)((int)param_1 + local_18 * 9 + 0x160),
                     (uint *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0x50));
        sVar1 = _strlen((char *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0x50));
        if (1 < sVar1) {
          param_1[0x33] = 1;
        }
        local_18 = local_18 + 1;
      }
      else if (**(int **)(param_1[0x5b3] + local_10 * 4) == 9) {
        FUN_0043ebd0((uint *)((int)param_1 + local_c * 9 + 0xd0),
                     (uint *)(*(int *)(param_1[0x5b3] + local_10 * 4) + 0x50));
        local_c = local_c + 1;
      }
    }
    local_10 = local_10 + 1;
  } while( true );
}
