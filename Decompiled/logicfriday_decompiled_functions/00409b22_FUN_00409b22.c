/* 00409b22 FUN_00409b22 */

void __cdecl FUN_00409b22(int param_1)

{
  uint uVar1;
  int iVar2;
  int iVar3;
  int local_28;
  uint local_1c;
  uint local_18;
  int local_14;
  uint local_10;
  int local_c;
  int local_8;
  
  SendMessageA(DAT_00452aac,0x111,0xcd,0);
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x1650); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0) {
      local_1c = 0xffffffff;
      switch(*(undefined4 *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4))) {
      case 0:
        local_1c = 0x3f4;
        break;
      case 1:
        iVar3 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar3 == 2) {
          local_1c = 0x3f5;
        }
        else if (iVar3 == 3) {
          local_1c = 0x3f6;
        }
        else if (iVar3 == 4) {
          local_1c = 0x3f7;
        }
        break;
      case 2:
        iVar3 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar3 == 2) {
          local_1c = 0x3f8;
        }
        else if (iVar3 == 3) {
          local_1c = 0x3f9;
        }
        else if (iVar3 == 4) {
          local_1c = 0x3fa;
        }
        break;
      case 3:
        local_1c = 0x430;
        break;
      case 5:
        local_1c = 0x3fc;
        break;
      case 6:
        iVar3 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar3 == 2) {
          local_1c = 0x3fd;
        }
        else if (iVar3 == 3) {
          local_1c = 0x3fe;
        }
        else if (iVar3 == 4) {
          local_1c = 0x3ff;
        }
        break;
      case 7:
        iVar3 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
        if (iVar3 == 2) {
          local_1c = 0x400;
        }
        else if (iVar3 == 3) {
          local_1c = 0x401;
        }
        else if (iVar3 == 4) {
          local_1c = 0x402;
        }
        break;
      case 8:
        local_1c = 0x438;
        break;
      case 9:
        local_1c = 0x439;
      }
      SendMessageA(DAT_00452a98,0x111,local_1c & 0xffff,0);
      if ((local_1c == 0x438) || (local_1c == 0x439)) {
        local_18 = local_1c;
        local_14 = *(int *)(param_1 + 0x3a4) + 0x50 + local_8 * 0xfc;
        __beginthread(FUN_0040a1fb,0,&local_18);
      }
      SendMessageA(DAT_00452a98,0x201,0,
                   *(uint *)(*(int *)(param_1 + 0x3a4) + 0xc0 + local_8 * 0xfc) & 0xffff |
                   *(int *)(*(int *)(param_1 + 0x3a4) + 0xc4 + local_8 * 0xfc) << 0x10);
    }
  }
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x1650); local_8 = local_8 + 1) {
    if ((*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4)) != 8) &&
       (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0)) {
      for (local_28 = 0; local_28 < *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
          local_28 = local_28 + 1) {
        if ((*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_28 * 4) == -2) ||
           (*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_28 * 4) == -1)) {
          if (*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_28 * 4) == -2) {
            local_1c = 0x408;
          }
          else {
            local_1c = 0x42a;
          }
          SendMessageA(DAT_00452a98,0x111,local_1c,0);
          iVar3 = *(int *)(param_1 + 0x3a4) + local_8 * 0xfc;
          uVar1 = *(uint *)(iVar3 + 0x6c + local_28 * 8);
          iVar3 = *(int *)(iVar3 + 0x70 + local_28 * 8);
          SendMessageA(DAT_00452a98,0x201,0,uVar1 - 10 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x111,0x42c,0);
          SendMessageA(DAT_00452a98,0x202,0,uVar1 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x200,0,uVar1 - 5 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x202,0,uVar1 - 10 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x111,0x42b,0);
        }
      }
    }
  }
  SendMessageA(DAT_00452a98,0x111,0x42c,0);
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x1650); local_8 = local_8 + 1) {
    if ((*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4)) != 8) &&
       (*(int *)(*(int *)(param_1 + 0x3a4) + 0x48 + local_8 * 0xfc) == 0)) {
      for (local_28 = 0; local_28 < *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_8 * 0xfc);
          local_28 = local_28 + 1) {
        if ((*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_28 * 4) != -2) &&
           (*(int *)(local_8 * 0xfc + *(int *)(param_1 + 0x3a4) + 0x1c + local_28 * 4) != -1)) {
          iVar3 = *(int *)(param_1 + 0x3a4) + local_8 * 0xfc;
          uVar1 = *(uint *)(iVar3 + 0x6c + local_28 * 8);
          iVar3 = *(int *)(iVar3 + 0x70 + local_28 * 8);
          iVar2 = *(int *)(*(int *)(param_1 + 0x3a4) + local_8 * 0xfc + 0x1c + local_28 * 4) * 0xfc;
          local_10 = *(uint *)(*(int *)(param_1 + 0x3a4) + 0xac + iVar2);
          local_c = *(int *)(*(int *)(param_1 + 0x3a4) + 0xb0 + iVar2);
          SendMessageA(DAT_00452a98,0x202,0,uVar1 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x200,0,uVar1 - 5 & 0xffff | iVar3 << 0x10);
          SendMessageA(DAT_00452a98,0x202,0,local_10 & 0xffff | local_c << 0x10);
        }
      }
    }
  }
  SendMessageA(DAT_00452a98,0x100,0xd,0);
  return;
}
