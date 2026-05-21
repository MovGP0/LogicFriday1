/* 00415979 FUN_00415979 */

char * FUN_00415979(undefined4 *param_1,char *param_2)

{
  char cVar1;
  char *pcVar2;
  char *pcVar3;
  char *local_10;
  char *local_c;
  
  pcVar2 = (char *)*param_1;
  pcVar3 = pcVar2;
  if (pcVar2 == (char *)0x0) {
    return (char *)0x0;
  }
  do {
    local_c = pcVar3;
    local_10 = param_2;
    do {
      cVar1 = *local_10;
      local_10 = local_10 + 1;
      if (cVar1 == *local_c) {
        if (*local_c == '\0') {
          local_c = (char *)0x0;
        }
        else {
          *local_c = '\0';
          local_c = local_c + 1;
        }
        *param_1 = local_c;
        return pcVar2;
      }
      pcVar3 = local_c + 1;
    } while (cVar1 != '\0');
  } while( true );
}
