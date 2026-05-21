/* 0040dd2e FUN_0040dd2e */

undefined4 __cdecl FUN_0040dd2e(int param_1)

{
  int iVar1;
  int local_c;
  
  iVar1 = *(int *)(param_1 + 200);
  if (5 < *(int *)(param_1 + 0xc4)) {
    switch(*(int *)(param_1 + 0xc4)) {
    case 6:
      if (9 < iVar1) {
        for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
          if ((10 < *(uint *)(param_1 + 4 + local_c * 4)) &&
             (*(uint *)(param_1 + 4 + local_c * 4) < 0x36)) {
            return 1;
          }
        }
      }
      break;
    case 7:
      if (6 < iVar1) {
        for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
          if ((0x14 < *(uint *)(param_1 + 4 + local_c * 4)) &&
             (*(uint *)(param_1 + 4 + local_c * 4) < 0x6c)) {
            return 1;
          }
        }
      }
      break;
    case 8:
      if (5 < iVar1) {
        for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
          if ((0x28 < *(uint *)(param_1 + 4 + local_c * 4)) &&
             (*(uint *)(param_1 + 4 + local_c * 4) < 0xd8)) {
            return 1;
          }
        }
      }
      break;
    case 9:
      if (2 < iVar1) {
        for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
          if ((0x50 < *(uint *)(param_1 + 4 + local_c * 4)) &&
             (*(uint *)(param_1 + 4 + local_c * 4) < 0x1b0)) {
            return 1;
          }
        }
      }
      break;
    case 10:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((0xa0 < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0x360)) {
          return 1;
        }
      }
      break;
    case 0xb:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((0xcc < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0x73a)) {
          return 1;
        }
      }
      break;
    case 0xc:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((0x32 < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0xfce)) {
          return 1;
        }
      }
      break;
    case 0xd:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((0x1e < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0x1fe2)) {
          return 1;
        }
      }
      break;
    case 0xe:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((0x14 < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0xffec)) {
          return 1;
        }
      }
      break;
    case 0xf:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((10 < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0x7ff6)) {
          return 1;
        }
      }
      break;
    case 0x10:
      for (local_c = 0; local_c < iVar1; local_c = local_c + 1) {
        if ((10 < *(uint *)(param_1 + 4 + local_c * 4)) &&
           (*(uint *)(param_1 + 4 + local_c * 4) < 0xfff6)) {
          return 1;
        }
      }
      break;
    default:
    }
  }
  return 0;
}
