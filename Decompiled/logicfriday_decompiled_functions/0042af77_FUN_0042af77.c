/* 0042af77 FUN_0042af77 */

void FUN_0042af77(undefined4 *param_1,undefined4 param_2)

{
  switch(param_2) {
  case 0x3f4:
    *param_1 = 0;
    param_1[6] = 1;
    break;
  case 0x3f5:
    *param_1 = 1;
    param_1[6] = 2;
    break;
  case 0x3f6:
    *param_1 = 1;
    param_1[6] = 3;
    break;
  case 0x3f7:
    *param_1 = 1;
    param_1[6] = 4;
    break;
  case 0x3f8:
    *param_1 = 2;
    param_1[6] = 2;
    break;
  case 0x3f9:
    *param_1 = 2;
    param_1[6] = 3;
    break;
  case 0x3fa:
    *param_1 = 2;
    param_1[6] = 4;
    break;
  case 0x3fc:
    *param_1 = 5;
    param_1[6] = 3;
    break;
  case 0x3fd:
    *param_1 = 6;
    param_1[6] = 2;
    break;
  case 0x3fe:
    *param_1 = 6;
    param_1[6] = 3;
    break;
  case 0x3ff:
    *param_1 = 6;
    param_1[6] = 4;
    break;
  case 0x400:
    *param_1 = 7;
    param_1[6] = 2;
    break;
  case 0x401:
    *param_1 = 7;
    param_1[6] = 3;
    break;
  case 0x402:
    *param_1 = 7;
    param_1[6] = 4;
    break;
  case 0x408:
    *param_1 = 0xb;
    param_1[6] = 0;
    param_1[0xf] = 0;
    break;
  case 0x42a:
    *param_1 = 10;
    param_1[6] = 0;
    param_1[0xf] = 0;
    break;
  case 0x430:
    *param_1 = 3;
    param_1[6] = 2;
    break;
  case 0x438:
    *param_1 = 8;
    param_1[6] = 0;
    param_1[0xf] = 0;
    FUN_0043ed39((char *)(param_1 + 0x14),&DAT_0044ad26);
    break;
  case 0x439:
    *param_1 = 9;
    param_1[6] = 1;
    FUN_0043ed39((char *)(param_1 + 0x14),&DAT_0044ad26);
  }
  return;
}
